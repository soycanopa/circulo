package core

import (
	"testing"

	"github.com/soycanopa/circulo/protocol"
)

func TestScriptOrder(t *testing.T) {
	script := Script()
	if len(script) < 4 {
		t.Fatalf("script length %d", len(script))
	}
	if script[0].Type != protocol.TypeHello || script[0].Seq != 1 {
		t.Fatalf("first %#v", script[0])
	}
	var sawLive bool
	for i, e := range script {
		if e.V != protocol.Version {
			t.Fatalf("version at %d", i)
		}
		if e.Seq != int64(i+1) {
			t.Fatalf("seq %d at %d", e.Seq, i)
		}
		if e.ID == LiveID {
			sawLive = true
			continue
		}
		if sawLive {
			t.Fatalf("history event after live reply: %#v", e)
		}
	}
	last := script[len(script)-1]
	if last.Type != protocol.TypeDone || last.ID != LiveID {
		t.Fatalf("last %#v", last)
	}
}
