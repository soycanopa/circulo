package omp

import (
	"encoding/base64"
	"encoding/json"
	"strings"
	"testing"
)

// chunkFrameJSON builds one rpc_chunk line carrying a base64 segment.
func chunkFrameJSON(t *testing.T, id string, index, count int, total int64, segment []byte) []byte {
	t.Helper()
	line, err := json.Marshal(chunkFrame{
		Type: "rpc_chunk", ChunkID: id, Index: index, Count: count,
		ByteLength: total, Data: base64.StdEncoding.EncodeToString(segment),
	})
	if err != nil {
		t.Fatal(err)
	}
	return line
}

// frameWithLimit builds a reader over lines with a 1 MiB line cap.
func frameWithLimit(t *testing.T, maxReassembled int64, lines ...[]byte) *frameReader {
	t.Helper()
	var b strings.Builder
	for _, l := range lines {
		b.Write(l)
		b.WriteByte('\n')
	}
	return newFrameReader(strings.NewReader(b.String()), 1<<20, maxReassembled)
}

func TestFrameReaderV1PassThrough(t *testing.T) {
	fr := frameWithLimit(t, 0,
		[]byte(`{"type":"ready"}`),
		[]byte(`# fixture comment`),
		[]byte(`{"type":"agent_start"}`),
	)
	for _, want := range []string{"ready", "agent_start"} {
		raw, err := fr.Read()
		if err != nil {
			t.Fatalf("Read: %v", err)
		}
		var e envelope
		if err := json.Unmarshal(raw, &e); err != nil {
			t.Fatal(err)
		}
		if string(e.Type) != want {
			t.Fatalf("got %q, want %q", e.Type, want)
		}
	}
	if _, err := fr.Read(); err == nil {
		t.Fatal("expected EOF")
	}
}

func TestFrameReaderReassemblesChunks(t *testing.T) {
	payload := []byte(`{"type":"response","command":"get_messages","success":true,"data":{"messages":[{"role":"user","content":"` + strings.Repeat("x", 500) + `"}]}}`)
	const parts = 4
	seg := (len(payload) + parts - 1) / parts
	var lines [][]byte
	for i := 0; i < parts; i++ {
		lo, hi := i*seg, (i+1)*seg
		if hi > len(payload) {
			hi = len(payload)
		}
		lines = append(lines, chunkFrameJSON(t, "rpc-1", i, parts, int64(len(payload)), payload[lo:hi]))
	}
	fr := frameWithLimit(t, 1<<20, lines...)
	raw, err := fr.Read()
	if err != nil {
		t.Fatal(err)
	}
	if string(raw) != string(payload) {
		t.Fatal("reassembled frame does not match original bytes")
	}
}

func TestFrameReaderRejectsBadChunkSequences(t *testing.T) {
	payload := []byte(`{"ok":true}`)
	c0 := chunkFrameJSON(t, "rpc-1", 0, 2, 10, payload)
	c1 := chunkFrameJSON(t, "rpc-1", 1, 2, 10, payload)

	t.Run("out of order", func(t *testing.T) {
		fr := frameWithLimit(t, 1<<20, c1, c0)
		if _, err := fr.Read(); err == nil {
			t.Fatal("expected index error")
		}
	})
	t.Run("interrupted by non-chunk frame", func(t *testing.T) {
		fr := frameWithLimit(t, 1<<20, c0, []byte(`{"type":"agent_start"}`))
		if _, err := fr.Read(); err == nil {
			t.Fatal("expected interruption error")
		}
	})
	t.Run("byteLength mismatch", func(t *testing.T) {
		fr := frameWithLimit(t, 1<<20, c0, chunkFrameJSON(t, "rpc-1", 1, 2, 999, payload))
		if _, err := fr.Read(); err == nil {
			t.Fatal("expected length mismatch error")
		}
	})
	t.Run("over reassembly ceiling", func(t *testing.T) {
		fr := frameWithLimit(t, 8, chunkFrameJSON(t, "rpc-1", 0, 1, 10, payload))
		if _, err := fr.Read(); err == nil {
			t.Fatal("expected ceiling error")
		}
	})
	t.Run("chunking disabled (v1 client)", func(t *testing.T) {
		fr := frameWithLimit(t, 0, c0)
		if _, err := fr.Read(); err == nil {
			t.Fatal("expected v1 rejection")
		}
	})
}

func TestFrameReaderRejectsInvalidUTF8(t *testing.T) {
	bad := []byte{'{', '"', 'k', '"', ':', 0xff, 0xfe, '}'}
	fr := frameWithLimit(t, 1<<20, chunkFrameJSON(t, "rpc-1", 0, 1, int64(len(bad)), bad))
	if _, err := fr.Read(); err == nil {
		t.Fatal("expected UTF-8 validation error")
	}
}

func TestParseReadyAndSupportsV2(t *testing.T) {
	rf, err := parseReady([]byte(`{"type":"ready","protocolVersion":1,"supportedProtocolVersions":[1,2],"maxFrameBytes":1048576,"maxReassembledFrameBytes":67108864}`))
	if err != nil {
		t.Fatal(err)
	}
	if !rf.supportsV2() {
		t.Fatal("expected v2 support")
	}
	if rf.MaxFrameBytes != 1<<20 {
		t.Fatalf("maxFrameBytes = %d", rf.MaxFrameBytes)
	}
	rf, err = parseReady([]byte(`{"type":"ready","protocolVersion":1,"supportedProtocolVersions":[1],"maxFrameBytes":1,"maxReassembledFrameBytes":0}`))
	if err != nil {
		t.Fatal(err)
	}
	if rf.supportsV2() {
		t.Fatal("expected no v2 support")
	}
}
