package opencode

import (
	"bufio"
	"io"
	"strings"
)

// Frame is one raw SSE data payload (the bytes after "data: ").
// The OpenCode /event stream uses data-only frames (no "event:" lines,
// verified against 1.18.25), but comment lines and multi-line data are
// handled per the SSE spec so a server change cannot wedge the reader.
type Frame []byte

func (f Frame) String() string { return string(f) }

// ReadFrames consumes an SSE stream and yields one Frame per event.
// Lines starting with ":" are comments (keep-alives) and are skipped.
// A frame is dispatched on the empty line that terminates it; a `data:` line
// without a terminating blank line at EOF is dropped (truncated write).
func ReadFrames(r io.Reader, out chan<- Frame) error {
	br := bufio.NewReaderSize(r, 64*1024)
	var data strings.Builder
	flush := func() {
		if data.Len() > 0 {
			out <- Frame(data.String())
			data.Reset()
		}
	}
	for {
		line, err := br.ReadString('\n')
		if len(line) > 0 {
			trimmed := strings.TrimRight(line, "\r\n")
			switch {
			case strings.HasPrefix(trimmed, ":"):
				// comment/keep-alive
			case strings.HasPrefix(trimmed, "data:"):
				v := strings.TrimPrefix(trimmed, "data:")
				if strings.HasPrefix(v, " ") {
					v = v[1:]
				}
				if data.Len() > 0 {
					data.WriteByte('\n')
				}
				data.WriteString(v)
			case trimmed == "":
				flush()
			default:
				// Unknown field (id:, retry:, event:…) — ignore per SSE spec.
			}
		}
		if err != nil {
			if err == io.EOF {
				// Do not flush a frame that was never terminated by a blank
				// line: the server may have died mid-write.
				return nil
			}
			return err
		}
	}
}
