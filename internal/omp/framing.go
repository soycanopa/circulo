package omp

import (
	"bufio"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"unicode/utf8"
)

// frameReader yields complete logical frames from the child's stdout.
//
// Protocol v1 stdout is one JSON object per line, capped at maxFrameBytes
// (1 MiB in practice). Protocol v2 encodes oversized frames as an
// uninterrupted run of rpc_chunk frames carrying base64 segments; this reader
// reassembles them with the validation rpc.md requires: matching chunkId,
// dense index order, consistent count/byteLength, and the reassembly ceiling.
// Comment lines (#) are a fixture convention, never produced by omp.
type frameReader struct {
	sc *bufio.Scanner

	// negotiated v2 limits; 0 disables chunk reassembly (v1 only).
	maxReassembled int64

	pendingChunkID string
	chunks         [][]byte
	chunkBytes     int64
	totalLen       int64
	expectedCount  int
}

// newFrameReader reads r line by line. maxLine bounds a physical v1 line;
// maxReassembled bounds a reassembled v2 frame (0 = chunking rejected).
func newFrameReader(r io.Reader, maxLine, maxReassembled int64) *frameReader {
	sc := bufio.NewScanner(r)
	sc.Buffer(make([]byte, 64*1024), int(maxLine))
	return &frameReader{sc: sc, maxReassembled: maxReassembled}
}

// Read returns the next complete logical frame. io.EOF marks orderly shutdown.
func (fr *frameReader) Read() (json.RawMessage, error) {
	for {
		line, err := fr.readLine()
		if err != nil {
			return nil, err
		}
		if len(line) == 0 || line[0] == '#' {
			continue
		}
		var env envelope
		if err := json.Unmarshal(line, &env); err != nil {
			return nil, fmt.Errorf("omp: undecodable stdout frame: %w", err)
		}
		if env.Type != frameChunk {
			frame, err := fr.finishChunk()
			if err != nil {
				return nil, err
			}
			if frame != nil {
				return frame, nil
			}
			return json.RawMessage(line), nil
		}
		if err := fr.addChunk(line); err != nil {
			return nil, err
		}
		if fr.chunkComplete() {
			frame, err := fr.assemble()
			fr.resetChunk()
			if err != nil {
				return nil, err
			}
			return frame, nil
		}
	}
}

func (fr *frameReader) readLine() ([]byte, error) {
	for fr.sc.Scan() {
		line := fr.sc.Bytes()
		// Copy: the scanner reuses its buffer.
		out := make([]byte, len(line))
		copy(out, line)
		return out, nil
	}
	if err := fr.sc.Err(); err != nil {
		// A v1 line over the physical cap surfaces as a too-long error.
		return nil, fmt.Errorf("omp: stdout read: %w", err)
	}
	return nil, io.EOF
}

func (fr *frameReader) addChunk(line []byte) error {
	var c chunkFrame
	if err := json.Unmarshal(line, &c); err != nil {
		return fmt.Errorf("omp: bad rpc_chunk: %w", err)
	}
	if fr.maxReassembled <= 0 {
		return fmt.Errorf("omp: got rpc_chunk but v2 framing is not enabled")
	}
	if fr.pendingChunkID == "" {
		fr.pendingChunkID = c.ChunkID
		if c.Count <= 0 {
			return fmt.Errorf("omp: chunk %s declares empty sequence", c.ChunkID)
		}
		fr.expectedCount = c.Count
		fr.chunks = make([][]byte, 0, c.Count)
	} else if c.ChunkID != fr.pendingChunkID {
		return fmt.Errorf("omp: interrupted chunk sequence %s -> %s", fr.pendingChunkID, c.ChunkID)
	}
	if c.Index != len(fr.chunks) {
		return fmt.Errorf("omp: chunk %s index %d out of order (want %d)", c.ChunkID, c.Index, len(fr.chunks))
	}
	if fr.totalLen == 0 {
		fr.totalLen = c.ByteLength
	} else if c.ByteLength != fr.totalLen {
		return fmt.Errorf("omp: chunk %s byteLength mismatch (%d then %d)", c.ChunkID, fr.totalLen, c.ByteLength)
	}
	raw, err := base64.StdEncoding.DecodeString(c.Data)
	if err != nil {
		return fmt.Errorf("omp: chunk %s base64: %w", c.ChunkID, err)
	}
	fr.chunkBytes += int64(len(raw))
	if fr.chunkBytes > fr.maxReassembled {
		return fmt.Errorf("omp: chunked frame exceeds reassembly limit (%d > %d)", fr.chunkBytes, fr.maxReassembled)
	}
	fr.chunks = append(fr.chunks, raw)
	return nil
}

func (fr *frameReader) chunkComplete() bool {
	return fr.expectedCount > 0 && len(fr.chunks) == fr.expectedCount
}

// finishChunk rejects a non-chunk frame arriving inside a chunk run.
func (fr *frameReader) finishChunk() (json.RawMessage, error) {
	if fr.pendingChunkID != "" {
		fr.resetChunk()
		return nil, fmt.Errorf("omp: chunk sequence interrupted by non-chunk frame")
	}
	return nil, nil
}

func (fr *frameReader) resetChunk() {
	fr.pendingChunkID = ""
	fr.chunks = nil
	fr.chunkBytes = 0
	fr.totalLen = 0
	fr.expectedCount = 0
}

func (fr *frameReader) assemble() (json.RawMessage, error) {
	joined := make([]byte, 0, fr.totalLen)
	for _, c := range fr.chunks {
		joined = append(joined, c...)
	}
	if int64(len(joined)) != fr.totalLen {
		return nil, fmt.Errorf("omp: chunked frame length mismatch (%d != %d)", len(joined), fr.totalLen)
	}
	if !utf8.Valid(joined) {
		return nil, fmt.Errorf("omp: chunked frame is not valid UTF-8")
	}
	return json.RawMessage(joined), nil
}

// parseReady decodes the startup ready frame.
func parseReady(line []byte) (*readyFrame, error) {
	var rf readyFrame
	if err := json.Unmarshal(line, &rf); err != nil {
		return nil, fmt.Errorf("omp: bad ready frame: %w", err)
	}
	if rf.Type != string(frameReady) {
		return nil, fmt.Errorf("omp: expected ready frame, got %q", rf.Type)
	}
	return &rf, nil
}

// supportsV2 reports whether the ready frame advertises protocol v2.
func (rf *readyFrame) supportsV2() bool {
	for _, v := range rf.SupportedProtocolVersions {
		if v == 2 {
			return true
		}
	}
	return false
}
