package orchestrator

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"sync"
	"testing"
	"time"

	"circulogo/internal/agent"
	"circulogo/internal/agent/protocol"
	"circulogo/internal/store"
)

// fakeAdapter is a scripted agent.Adapter for orchestrator tests.
type fakeAdapter struct {
	mu        sync.Mutex
	starts    int
	stops     int
	fail      error // if set, Start returns it
	startHook func()      // if set, Start blocks on it after signaling started
	events    chan protocol.Envelope
}

func newFakeAdapter() *fakeAdapter {
	return &fakeAdapter{events: make(chan protocol.Envelope, 64)}
}

func (f *fakeAdapter) Start(_ context.Context) error {
	f.mu.Lock()
	f.starts++
	hook := f.startHook
	fail := f.fail
	f.mu.Unlock()
	if hook != nil {
		hook()
	}
	if fail != nil {
		return fail
	}
	f.mu.Lock()
	defer f.mu.Unlock()
	env, _ := protocol.NewEnvelope(protocol.EventAdapterStatus, protocol.AdapterStatus{
		ProjectID: "x", State: protocol.AdapterRunning,
	})
	f.events <- env
	return nil
}

func (f *fakeAdapter) Stop(_ context.Context) error {
	f.mu.Lock()
	defer f.mu.Unlock()
	f.stops++
	return nil
}

func (f *fakeAdapter) Events() <-chan protocol.Envelope { return f.events }

// Unused contract methods panic — orchestrator must not call them.
func (f *fakeAdapter) Sessions(_ context.Context) ([]protocol.Session, error) {
	panic("unexpected")
}
func (f *fakeAdapter) CreateSession(_ context.Context, _ string) (protocol.Session, error) {
	panic("unexpected")
}
func (f *fakeAdapter) RenameSession(_ context.Context, _, _ string) error { panic("unexpected") }
func (f *fakeAdapter) DeleteSession(_ context.Context, _ string) error    { panic("unexpected") }
func (f *fakeAdapter) Messages(_ context.Context, _ string, _ int) ([]agent.HydratedMessage, error) {
	panic("unexpected")
}
func (f *fakeAdapter) Prompt(_ context.Context, _ string, _ protocol.PromptRequest) error {
	panic("unexpected")
}
func (f *fakeAdapter) Abort(_ context.Context, _ string) error { panic("unexpected") }
func (f *fakeAdapter) ReplyPermission(_ context.Context, _, _, _ string) error {
	panic("unexpected")
}
func (f *fakeAdapter) Meta(_ context.Context) (protocol.Meta, error) { panic("unexpected") }

// fakeFactory wires new fakeAdapters and remembers them by project path.
type fakeFactory struct {
	mu       sync.Mutex
	adapters map[string]*fakeAdapter
}

func newFakeFactory() *fakeFactory {
	return &fakeFactory{adapters: map[string]*fakeAdapter{}}
}

func (ff *fakeFactory) build(cfg store.Project) (agent.Adapter, error) {
	ff.mu.Lock()
	defer ff.mu.Unlock()
	a := newFakeAdapter()
	// Convention: port 1 URLs stand in for "unreachable backend" scenarios.
	if strings.HasSuffix(cfg.URL, ":1") {
		a.fail = errBackendDown
	}
	ff.adapters[cfg.Path] = a
	return a, nil
}

func (ff *fakeFactory) get(path string) *fakeAdapter {
	ff.mu.Lock()
	defer ff.mu.Unlock()
	return ff.adapters[path]
}

func newTestOrchestrator(t *testing.T, ff *fakeFactory) (*Orchestrator, string) {
	t.Helper()
	path := t.TempDir() + "/settings.json"
	o := New(store.New(path), ff.build)
	return o, path
}

func waitStatus(t *testing.T, ch <-chan protocol.Envelope, state string) {
	t.Helper()
	deadline := time.After(3 * time.Second)
	for {
		select {
		case e := <-ch:
			if e.Type == protocol.EventAdapterStatus {
				var st protocol.AdapterStatus
				if err := json.Unmarshal(e.Payload, &st); err != nil {
					t.Fatal(err)
				}
				if st.State == state {
					return
				}
			}
		case <-deadline:
			t.Fatalf("no adapter.status %s arrived", state)
		}
	}
}

func TestLoadStartsPersistedProjects(t *testing.T) {
	ff := newFakeFactory()
	o, path := newTestOrchestrator(t, ff)
	st := store.New(path)
	if err := st.Save(store.Settings{Projects: []store.Project{
		{ID: store.ProjectID("/tmp/a"), Path: "/tmp/a", Mode: "attach", URL: "http://127.0.0.1:4096"},
	}}); err != nil {
		t.Fatal(err)
	}

	ch, cancel := o.Subscribe()
	defer cancel()
	if err := o.Load(context.Background()); err != nil {
		t.Fatal(err)
	}
	waitStatus(t, ch, protocol.AdapterRunning)

	ps := o.Projects()
	if len(ps) != 1 || ps[0].Path != "/tmp/a" || ps[0].Status != protocol.AdapterRunning {
		t.Errorf("projects = %+v", ps)
	}
}

func TestAddRemoveProjectPersists(t *testing.T) {
	ff := newFakeFactory()
	o, path := newTestOrchestrator(t, ff)
	if err := o.Load(context.Background()); err != nil {
		t.Fatal(err)
	}

	dir := t.TempDir()
	pv, err := o.AddProject(context.Background(), dir, "attach", "http://127.0.0.1:9")
	if err != nil {
		t.Fatal(err)
	}
	if pv.Status != protocol.AdapterStarting {
		t.Errorf("new project status = %s, want starting", pv.Status)
	}

	// Persisted?
	set, err := store.New(path).Load()
	if err != nil || len(set.Projects) != 1 {
		t.Fatalf("settings = %+v err=%v", set, err)
	}

	// Adding the same dir again is rejected (stable ids).
	if _, err := o.AddProject(context.Background(), dir, "attach", "http://x"); err == nil {
		t.Fatal("expected duplicate rejection")
	}

	if err := o.RemoveProject(context.Background(), pv.ID); err != nil {
		t.Fatal(err)
	}
	set, _ = store.New(path).Load()
	if len(set.Projects) != 0 {
		t.Errorf("settings after remove = %+v", set)
	}
}

func TestAddProjectManagedRequiresExistingDir(t *testing.T) {
	o, _ := newTestOrchestrator(t, newFakeFactory())
	if _, err := o.AddProject(context.Background(), "/definitely/not/there", "managed", ""); err == nil {
		t.Fatal("expected error for missing dir")
	}
}

func TestAdapterFailureSurfacesAsErrorStatus(t *testing.T) {
	ff := newFakeFactory()
	o, path := newTestOrchestrator(t, ff)
	if err := store.New(path).Save(store.Settings{Projects: []store.Project{
		{ID: store.ProjectID("/tmp/broken"), Path: "/tmp/broken", Mode: "attach", URL: "http://127.0.0.1:1"},
	}}); err != nil {
		t.Fatal(err)
	}
	ch, cancel := o.Subscribe()
	defer cancel()
	if err := o.Load(context.Background()); err != nil {
		t.Fatal(err)
	}
	waitStatus(t, ch, protocol.AdapterError)

	ps := o.Projects()
	if ps[0].Status != protocol.AdapterError {
		t.Errorf("status = %s, want error", ps[0].Status)
	}
}

func TestSubscribeReplaysRecentEvents(t *testing.T) {
	ff := newFakeFactory()
	o, path := newTestOrchestrator(t, ff)
	if err := store.New(path).Save(store.Settings{Projects: []store.Project{
		{ID: store.ProjectID("/tmp/r"), Path: "/tmp/r", Mode: "attach", URL: "http://127.0.0.1:1"},
	}}); err != nil {
		t.Fatal(err)
	}
	if err := o.Load(context.Background()); err != nil {
		t.Fatal(err)
	}
	// Wait for the first adapter to have emitted, then subscribe late.
	deadline := time.Now().Add(2 * time.Second)
	for {
		if o.Projects()[0].Status == protocol.AdapterRunning || time.Now().After(deadline) {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}

	sub, cancel := o.Subscribe()
	defer cancel()
	select {
	case env := <-sub:
		if env.Type != protocol.EventAdapterStatus {
			t.Fatalf("replayed %s, want adapter.status", env.Type)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("no replayed event for late subscriber")
	}
}

func TestShutdownStopsAllAdapters(t *testing.T) {
	ff := newFakeFactory()
	o, path := newTestOrchestrator(t, ff)
	projects := []store.Project{}
	for _, p := range []string{"/tmp/s1", "/tmp/s2"} {
		projects = append(projects, store.Project{ID: store.ProjectID(p), Path: p, Mode: "attach", URL: "http://127.0.0.1:1"})
	}
	if err := store.New(path).Save(store.Settings{Projects: projects}); err != nil {
		t.Fatal(err)
	}
	if err := o.Load(context.Background()); err != nil {
		t.Fatal(err)
	}
	// Adapters register asynchronously; poll until both exist.
	deadline := time.Now().Add(2 * time.Second)
	for {
		ff.mu.Lock()
		n := len(ff.adapters)
		ff.mu.Unlock()
		if n == 2 || time.Now().After(deadline) {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	o.Shutdown()
	for p, a := range ff.adapters {
		if a.stops == 0 {
			t.Errorf("adapter for %s was not stopped", p)
		}
	}
}


var errBackendDown = errors.New("fake: backend down")

var _ = fmt.Sprintf

// Regression: commands arriving while an adapter is still starting must get
// NotReadyError (→ relay 503), not a nil-client panic that killed the app.
func TestAdapterOfGatesUntilReady(t *testing.T) {
	release := make(chan struct{})
	started := make(chan struct{})
	o := New(store.New(t.TempDir()+"/settings.json"), func(cfg store.Project) (agent.Adapter, error) {
		a := newFakeAdapter()
		a.startHook = func() {
			close(started)
			<-release
		}
		return a, nil
	})
	dir := t.TempDir()
	pv, err := o.AddProject(context.Background(), dir, "attach", "http://127.0.0.1:9")
	if err != nil {
		t.Fatal(err)
	}
	<-started // adapter is registered but Start is blocked

	if _, err := o.AdapterOf(pv.ID); err == nil {
		t.Fatal("expected NotReadyError while Start is blocked")
	} else {
		var nr *NotReadyError
		if !errorsAs(err, &nr) {
			t.Fatalf("err = %v, want NotReadyError", err)
		}
	}

	close(release)
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		if _, err := o.AdapterOf(pv.ID); err == nil {
			return // ready
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Fatal("adapter never became ready")
}

func errorsAs(err error, target any) bool { return errors.As(err, target) }
