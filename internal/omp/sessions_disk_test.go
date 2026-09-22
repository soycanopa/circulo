package omp

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestEncodeCwdBucketTempRoot(t *testing.T) {
	dir := t.TempDir()
	// macOS TMPDIR is /var/folders/... but resolves to /private/var/...; the
	// encoder canonicalizes both sides so the bucket must use the temp-root
	// spelling.
	bucket, err := encodeCwdBucket(dir)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasPrefix(bucket, "-tmp-") {
		t.Fatalf("bucket = %q, want -tmp- prefix", bucket)
	}
	if strings.Contains(bucket, "/") {
		t.Fatalf("bucket contains separators: %q", bucket)
	}
}

func TestEncodeCwdBucketAbsolute(t *testing.T) {
	// /usr/lib exists, is not under home or the temp root.
	bucket, err := encodeCwdBucket("/usr/lib")
	if err != nil {
		t.Fatal(err)
	}
	if bucket != "--usr-lib--" {
		t.Fatalf("bucket = %q", bucket)
	}
}

// writeSessionFile writes one session JSONL in the current format: a
// 256-byte-padded title slot, the header, then entries.
func writeSessionFile(t *testing.T, bucket, name, title, id string, firstUser string) string {
	t.Helper()
	slot := `{"type":"title","title":"` + title + `"}`
	if len(slot) < 256 {
		slot += strings.Repeat(" ", 256-len(slot))
	}
	header := `{"type":"session","version":3,"id":"` + id + `","timestamp":"2026-02-16T10:20:30.000Z","cwd":"/work/x"}`
	var b strings.Builder
	b.WriteString(slot + "\n")
	b.WriteString(header + "\n")
	if firstUser != "" {
		entry := map[string]any{
			"type": "message",
			"message": map[string]any{
				"role": "user",
				"content": []map[string]any{
					{"type": "text", "text": firstUser},
				},
			},
		}
		raw, _ := json.Marshal(entry)
		b.WriteString(string(raw) + "\n")
	}
	path := filepath.Join(t.TempDir(), bucket)
	// Caller wants a specific bucket under the fake home: build relative to
	// an explicit root when provided via the bucket arg.
	if !filepath.IsAbs(bucket) {
		path = filepath.Join(fakeHomePath, sessionRoot, bucket, name)
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(b.String()), 0o644); err != nil {
		t.Fatal(err)
	}
	return path
}

// fakeHomePath is set by tests that install a fake HOME.
var fakeHomePath string

func TestDiscoverSessions(t *testing.T) {
	fakeHome := t.TempDir()
	fakeHomePath = fakeHome
	t.Setenv("HOME", fakeHome)

	// A real directory: the encoder canonicalizes it, so the test must write
	// into the bucket that DiscoverSessions will compute.
	dir := t.TempDir()
	bucket, err := encodeCwdBucket(dir)
	if err != nil {
		t.Fatal(err)
	}

	// Newest session (touched last): titled via slot.
	p1 := writeSessionFile(t, bucket, "100_a.jsonl", "Titled session", "aaaa1111", "")
	// Older session: no title → first user text becomes the label.
	p2 := writeSessionFile(t, bucket, "050_b.jsonl", "", "bbbb2222", "fix the parser\nsecond line")
	// Legacy header-first file with a header title.
	legacy := filepath.Join(fakeHome, sessionRoot, bucket, "010_c.jsonl")
	if err := os.MkdirAll(filepath.Dir(legacy), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(legacy, []byte(
		`{"type":"session","version":2,"id":"cccc3333","timestamp":"2026-01-01T00:00:00.000Z","cwd":"`+dir+`","title":"Legacy"}`+"\n"+
			`{"type":"message","message":{"role":"assistant","content":[{"type":"text","text":"hi"}]}}`+"\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	// Damaged file: skipped, never hides the rest.
	if err := os.WriteFile(filepath.Join(fakeHome, sessionRoot, bucket, "200_d.jsonl"), []byte("garbage"), 0o644); err != nil {
		t.Fatal(err)
	}
	// mtime ordering: p2 oldest, legacy newer, p1 newest.
	base := time.Now()
	for _, c := range []struct {
		path string
		at   time.Time
	}{{p2, base.Add(-3 * time.Hour)}, {legacy, base.Add(-1 * time.Hour)}, {p1, base}} {
		if err := os.Chtimes(c.path, c.at, c.at); err != nil {
			t.Fatal(err)
		}
	}

	sessions, err := DiscoverSessions(dir)
	if err != nil {
		t.Fatal(err)
	}
	if len(sessions) != 3 {
		t.Fatalf("want 3 sessions, got %d: %+v", len(sessions), sessions)
	}
	if sessions[0].Session.ID != "aaaa1111" || sessions[0].Session.Title != "Titled session" {
		t.Fatalf("newest = %+v", sessions[0].Session)
	}
	if sessions[1].Session.ID != "cccc3333" || sessions[1].Session.Title != "Legacy" {
		t.Fatalf("second = %+v", sessions[1].Session)
	}
	if sessions[2].Session.ID != "bbbb2222" || sessions[2].Session.Title != "fix the parser" {
		t.Fatalf("oldest = %+v (want first-user-text fallback)", sessions[2].Session)
	}
	if sessions[2].Session.TimeCreated == 0 {
		t.Fatal("header timestamp not parsed")
	}

	path, err := FindSessionFile(dir, "bbbb2222")
	if err != nil || filepath.Base(path) != "050_b.jsonl" {
		t.Fatalf("FindSessionFile = %q, %v", path, err)
	}
	if _, err := FindSessionFile(dir, "nope"); err == nil {
		t.Fatal("expected missing session error")
	}
}
