package protocol

import (
	"bytes"
	"io"
	"testing"
)

func TestRoundTrip(t *testing.T) {
	var buf bytes.Buffer
	in := Envelope{V: Version, Type: TypeDelta, Seq: 2, ID: "m1", Role: "assistant", Text: "hi"}
	if err := Write(&buf, in); err != nil {
		t.Fatal(err)
	}
	got, err := NewReader(&buf).Next()
	if err != nil {
		t.Fatal(err)
	}
	if got != in {
		t.Fatalf("got %#v", got)
	}
}

func TestRejectsOtherVersion(t *testing.T) {
	err := Write(io.Discard, Envelope{V: 2, Type: TypeHello, Seq: 1})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestUnknownTypePasses(t *testing.T) {
	var buf bytes.Buffer
	in := Envelope{V: Version, Type: "future.event", Seq: 4}
	if err := Write(&buf, in); err != nil {
		t.Fatal(err)
	}
	got, err := NewReader(&buf).Next()
	if err != nil {
		t.Fatal(err)
	}
	if got.Type != "future.event" {
		t.Fatalf("type %q", got.Type)
	}
}
