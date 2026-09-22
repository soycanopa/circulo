package omp

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"circulogo/internal/agent/protocol"
)

// omp stores file sessions under a canonical-cwd bucket:
//
//	~/.omp/agent/sessions/<encoded-cwd>/<timestamp>_<sessionId>.jsonl
//
// (docs/session.md, oh-my-pi). The RPC protocol has no list command, so the
// adapter discovers sessions the same way omp's own pickers do: scan the
// bucket and parse a small prefix of each file.
const sessionRoot = ".omp/agent/sessions"

// encodeCwdBucket reproduces omp's cwd-bucket name for the canonical dir:
// "-<relative>" under home, "-tmp-<relative>" under the temp root,
// "--<encoded-absolute>--" otherwise; path separators become "-".
func encodeCwdBucket(dir string) (string, error) {
	canonical, err := filepath.EvalSymlinks(dir)
	if err != nil {
		return "", fmt.Errorf("omp: resolve project dir: %w", err)
	}
	home, err := os.UserHomeDir()
	if err == nil {
		if homeCanon, herr := filepath.EvalSymlinks(home); herr == nil && within(canonical, homeCanon) {
			return "-" + flatten(strings.TrimPrefix(canonical, homeCanon+string(filepath.Separator))), nil
		}
	}
	tmp := os.TempDir()
	if tmpCanon, terr := filepath.EvalSymlinks(tmp); terr == nil && within(canonical, tmpCanon) {
		return "-tmp-" + flatten(strings.TrimPrefix(canonical, tmpCanon+string(filepath.Separator))), nil
	}
	abs := strings.TrimPrefix(canonical, string(filepath.Separator))
	return "--" + flatten(abs) + "--", nil
}

// within reports whether dir equals or lives under root.
func within(dir, root string) bool {
	return dir == root || strings.HasPrefix(dir, root+string(filepath.Separator))
}

// flatten replaces path separators (and any remaining slashes) with "-".
func flatten(rel string) string {
	return strings.ReplaceAll(strings.ReplaceAll(rel, string(filepath.Separator), "-"), "/", "-")
}

// osRemove is the session-file delete hook (tests swap it out).
var osRemove = os.Remove

// SessionFile is one discovered session JSONL.
type SessionFile struct {
	Session protocol.Session
	Path    string
}

// DiscoverSessions lists the project bucket, newest first. Each file gets a
// 4 KiB prefix read: the fixed-width title slot, the session header, and —
// when no title exists — the first user text as a fallback label. Parse
// failures skip the file: one damaged session must not hide the rest.
func DiscoverSessions(dir string) ([]SessionFile, error) {
	bucket, err := encodeCwdBucket(dir)
	if err != nil {
		return nil, err
	}
	home, err := os.UserHomeDir()
	if err != nil {
		return nil, fmt.Errorf("omp: home dir: %w", err)
	}
	entries, err := os.ReadDir(filepath.Join(home, sessionRoot, bucket))
	if errors.Is(err, os.ErrNotExist) {
		return nil, nil // fresh project: no sessions yet
	}
	if err != nil {
		return nil, fmt.Errorf("omp: session bucket: %w", err)
	}

	out := make([]SessionFile, 0, len(entries))
	for _, e := range entries {
		if e.IsDir() || !strings.HasSuffix(e.Name(), ".jsonl") {
			continue
		}
		path := filepath.Join(home, sessionRoot, bucket, e.Name())
		s, ok := parseSessionPrefix(path)
		if !ok {
			continue
		}
		out = append(out, SessionFile{Session: s, Path: path})
	}
	sort.Slice(out, func(i, j int) bool {
		return out[i].Session.TimeUpdated > out[j].Session.TimeUpdated
	})
	return out, nil
}

// FindSessionFile resolves one session id to its file path.
func FindSessionFile(dir, sessionID string) (string, error) {
	sessions, err := DiscoverSessions(dir)
	if err != nil {
		return "", err
	}
	for _, s := range sessions {
		if s.Session.ID == sessionID {
			return s.Path, nil
		}
	}
	return "", fmt.Errorf("omp: session %s not found in project %s", sessionID, dir)
}

// headerLine is the logical first entry of a session file.
type headerLine struct {
	Type      string `json:"type"`
	ID        string `json:"id"`
	Timestamp string `json:"timestamp"` // RFC3339
	CWD       string `json:"cwd"`
	Title     string `json:"title"`
}

// titleSlot is the fixed-width first line of current session files.
type titleSlot struct {
	Type  string `json:"type"`
	Title string `json:"title"`
}

// parseSessionPrefix reads at most 4 KiB and extracts the sidebar view.
func parseSessionPrefix(path string) (protocol.Session, bool) {
	f, err := os.Open(path)
	if err != nil {
		return protocol.Session{}, false
	}
	defer f.Close()

	prefix := make([]byte, 4<<10)
	n, _ := f.Read(prefix)
	prefix = prefix[:n]
	info, err := f.Stat()
	if err != nil {
		return protocol.Session{}, false
	}

	var (
		header    headerLine
		title     string
		firstUser string
		haveTitle bool
	)
	for _, line := range strings.Split(string(prefix), "\n") {
		line = strings.TrimSpace(line)
		if line == "" || line[0] != '{' {
			continue
		}
		if !haveTitle {
			var slot titleSlot
			if json.Unmarshal([]byte(line), &slot) == nil && slot.Type == "title" {
				title = slot.Title
				haveTitle = true
				continue
			}
		}
		if header.ID == "" {
			if json.Unmarshal([]byte(line), &header) == nil && header.Type == "session" {
				continue
			}
			header = headerLine{} // that line was not the header
		} else if firstUser == "" {
			firstUser = firstUserText(line)
			if title != "" && firstUser != "" {
				break
			}
		}
	}
	if header.ID == "" {
		return protocol.Session{}, false
	}
	if title == "" {
		title = header.Title
	}
	if title == "" {
		title = firstUser
	}
	created, _ := time.Parse(time.RFC3339, header.Timestamp)
	var createdMs int64
	if !created.IsZero() {
		createdMs = created.UnixMilli()
	}
	return protocol.Session{
		ID:          header.ID,
		Title:       title,
		Directory:   header.CWD,
		TimeCreated: createdMs,
		TimeUpdated: info.ModTime().UnixMilli(),
	}, true
}

// firstUserText scrapes the first user message text from one JSONL entry line.
// Entry shapes vary across omp versions; anything unexpected yields "".
func firstUserText(line string) string {
	var probe struct {
		Message json.RawMessage `json:"message"`
	}
	if json.Unmarshal([]byte(line), &probe) != nil || len(probe.Message) == 0 {
		return ""
	}
	var msg struct {
		Role    string          `json:"role"`
		Content json.RawMessage `json:"content"`
	}
	if json.Unmarshal(probe.Message, &msg) != nil || msg.Role != "user" {
		return ""
	}
	if len(msg.Content) > 0 && msg.Content[0] == '"' {
		var s string
		if json.Unmarshal(msg.Content, &s) == nil {
			return firstLine(s)
		}
		return ""
	}
	var blocks []struct {
		Type string `json:"type"`
		Text string `json:"text"`
	}
	if json.Unmarshal(msg.Content, &blocks) != nil {
		return ""
	}
	for _, b := range blocks {
		if b.Type == "text" && b.Text != "" {
			return firstLine(b.Text)
		}
	}
	return ""
}

func firstLine(s string) string {
	if i := strings.IndexByte(s, '\n'); i >= 0 {
		return s[:i]
	}
	return s
}
