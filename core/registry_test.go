package core

import (
	"context"
	"net"
	"os"
	"path/filepath"
	"sync"
	"testing"
	"time"

	"github.com/soycanopa/circulo/driver"
	"github.com/soycanopa/circulo/persistence"
	"github.com/soycanopa/circulo/protocol"
)

type fakeDriver struct {
	mu       sync.Mutex
	dir      string
	prompts  []string
	delivery string
	model    driver.Model
	aborted  bool
	ev       chan driver.Event
}

func (f *fakeDriver) CreateSession(_ context.Context, dir, _ string) (string, error) {
	f.mu.Lock()
	f.dir = dir
	f.mu.Unlock()
	return "ses_fake", nil
}

func (f *fakeDriver) Prompt(_ context.Context, _ string, text, delivery string) error {
	f.mu.Lock()
	f.prompts = append(f.prompts, text)
	f.delivery = delivery
	f.mu.Unlock()
	return nil
}

func (f *fakeDriver) Models(context.Context, string) ([]driver.Model, error) {
	return []driver.Model{{
		ID: "big-pickle", ProviderID: "opencode", Name: "Big Pickle", Variants: []string{"low", "high"},
	}}, nil
}

func (f *fakeDriver) DefaultModel(context.Context, string) (driver.Model, error) {
	return driver.Model{ID: "big-pickle", ProviderID: "opencode", Variant: "low"}, nil
}

func (f *fakeDriver) SessionModel(context.Context, string) (driver.Model, error) {
	return driver.Model{}, nil
}

func (f *fakeDriver) SetModel(_ context.Context, _ string, model driver.Model) error {
	f.mu.Lock()
	f.model = model
	f.mu.Unlock()
	return nil
}

func (f *fakeDriver) Abort(context.Context, string) error {
	f.mu.Lock()
	f.aborted = true
	f.mu.Unlock()
	return nil
}

func (f *fakeDriver) Watch(ctx context.Context, fn func(driver.Event)) error {
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case e := <-f.ev:
			fn(e)
		}
	}
}

func TestRegistryProjectAndPrompt(t *testing.T) {
	dir := t.TempDir()
	db, err := persistence.Open(filepath.Join(t.TempDir(), "c.db"))
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	fake := &fakeDriver{ev: make(chan driver.Event, 4)}
	reg := NewRegistry(db, fake, nil)
	ln, err := Listen("127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	go reg.Serve(ctx, ln)

	conn, err := net.Dial("tcp", ln.Addr().String())
	if err != nil {
		t.Fatal(err)
	}
	defer conn.Close()
	rd := protocol.NewReader(conn)
	hello, err := rd.Next()
	if err != nil || hello.Type != protocol.TypeHello {
		t.Fatalf("hello %#v %v", hello, err)
	}
	if err := protocol.Write(conn, protocol.Envelope{V: 1, Type: protocol.TypeAdd, Seq: 1, Path: dir}); err != nil {
		t.Fatal(err)
	}
	var sawSession bool
	deadline := time.Now().Add(3 * time.Second)
	for time.Now().Before(deadline) && !sawSession {
		conn.SetReadDeadline(deadline)
		e, err := rd.Next()
		if err != nil {
			t.Fatal(err)
		}
		if e.Type == protocol.TypeSession && e.ID == "ses_fake" {
			sawSession = true
		}
	}
	if !sawSession {
		t.Fatal("no session")
	}
	if err := protocol.Write(conn, protocol.Envelope{V: 1, Type: protocol.TypePrompt, Seq: 2, ID: "ses_fake", Text: "hello"}); err != nil {
		t.Fatal(err)
	}
	fake.ev <- driver.Event{Kind: driver.KindDelta, SessionID: "ses_fake", MessageID: "msg_1", Text: "Hi"}
	var user, agent bool
	var seen []string
	deadline = time.Now().Add(3 * time.Second)
	for time.Now().Before(deadline) && (!user || !agent) {
		conn.SetReadDeadline(deadline)
		e, err := rd.Next()
		if err != nil {
			t.Fatalf("after %v: %v prompts %v", seen, err, fake.prompts)
		}
		seen = append(seen, e.Type+":"+e.Role+":"+e.Text)
		if e.Type == protocol.TypeDelta && e.Role == "user" && e.Text == "hello" {
			user = true
		}
		if e.Type == protocol.TypeDelta && e.Text == "Hi" {
			agent = true
		}
	}
	if !user || !agent {
		t.Fatalf("user %v agent %v seen %v prompts %v", user, agent, seen, fake.prompts)
	}
	if _, err := os.Stat(dir); err != nil {
		t.Fatal(err)
	}
}

func TestRegistryModelAndAbort(t *testing.T) {
	dir := t.TempDir()
	db, err := persistence.Open(filepath.Join(t.TempDir(), "c.db"))
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	fake := &fakeDriver{ev: make(chan driver.Event, 1)}
	reg := NewRegistry(db, fake, nil)
	if err := reg.AddProject(context.Background(), dir); err != nil {
		t.Fatal(err)
	}
	if err := reg.UseModel(context.Background(), "ses_fake", driver.Model{ID: "big-pickle", ProviderID: "opencode", Variant: "high"}); err != nil {
		t.Fatal(err)
	}
	if fake.model.Variant != "high" {
		t.Fatalf("%#v", fake.model)
	}
	if err := reg.Prompt(context.Background(), "ses_fake", "later", "queue"); err != nil {
		t.Fatal(err)
	}
	if fake.delivery != "queue" {
		t.Fatalf("delivery %q", fake.delivery)
	}
	if err := reg.Abort(context.Background(), "ses_fake"); err != nil || !fake.aborted {
		t.Fatalf("abort %v %v", err, fake.aborted)
	}
}
