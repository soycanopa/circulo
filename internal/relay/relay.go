// Package relay is the same-origin HTTP API the webview talks to. It mounts at
// route /agent inside the Wails asset server (docs/trd.md §4) and translates
// HTTP ⇄ orchestrator calls. It holds no business logic.
package relay

import (
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"strconv"
	"strings"
	"time"

	"circulogo/internal/agent"
	"circulogo/internal/agent/protocol"
	"circulogo/internal/orchestrator"
	"circulogo/internal/term"
)

// Server implements http.Handler for the agent API. Wails mounts it at route
// /agent and hands it paths with the prefix ALREADY stripped (assetserver.go
// TrimPrefix) — handlers here therefore see "/projects", "/sse", … For direct
// mounting (the debug listener) wrap with http.StripPrefix("/agent", …).
type Server struct {
	orch *orchestrator.Orchestrator
}

// New builds the relay API over an orchestrator.
func New(orch *orchestrator.Orchestrator) *Server {
	return &Server{orch: orch}
}

func (s *Server) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	path := strings.TrimPrefix(r.URL.Path, "/")
	if path == "" {
		http.NotFound(w, r)
		return
	}

	switch {
	case path == "sse" && r.Method == http.MethodGet:
		s.handleSSE(w, r)
		return
	case path == "projects" && r.Method == http.MethodGet:
		s.writeJSON(w, http.StatusOK, s.orch.Projects())
		return
	case path == "projects" && r.Method == http.MethodPost:
		s.handleAddProject(w, r)
		return
	}

	// /projects/{id}/…
	section, tail := take(path)
	if section != "projects" {
		http.NotFound(w, r)
		return
	}
	id, rest := take(tail)
	if id == "" {
		http.NotFound(w, r)
		return
	}

	switch {
	case rest == "" && r.Method == http.MethodGet:
		pv, err := s.orch.Project(id)
		s.writeJSONOrErr(w, pv, err)
		return
	case rest == "" && r.Method == http.MethodDelete:
		if err := s.orch.RemoveProject(r.Context(), id); err != nil {
			s.writeErr(w, err)
			return
		}
		s.writeJSON(w, http.StatusOK, map[string]bool{"ok": true})
		return
	case rest == "meta" && r.Method == http.MethodGet:
		s.withAdapter(w, id, func(a agent.Adapter) {
			meta, err := a.Meta(r.Context())
			s.writeJSONOrErr(w, meta, err)
		})
		return
	case rest == "access" && r.Method == http.MethodGet:
		s.withAdapter(w, id, func(a agent.Adapter) {
			meta, err := a.Meta(r.Context())
			if err != nil {
				s.writeErr(w, err)
				return
			}
			s.writeJSON(w, http.StatusOK, meta.Access)
		})
		return
	case rest == "access" && r.Method == http.MethodPost:
		var body protocol.SetAccessRequest
		if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 4096)).Decode(&body); err != nil {
			s.writeErr(w, badRequest("access body: %v", err))
			return
		}
		if err := s.orch.SetAccess(r.Context(), id, body.Mode); err != nil {
			s.writeErr(w, err)
			return
		}
		s.writeJSON(w, http.StatusOK, map[string]bool{"ok": true})
		return
	case rest == "sessions" && r.Method == http.MethodGet:
		s.withAdapter(w, id, func(a agent.Adapter) {
			ss, err := a.Sessions(r.Context())
			s.writeJSONOrErr(w, ss, err)
		})
		return
	case rest == "sessions" && r.Method == http.MethodPost:
		s.handleCreateSession(w, r, id)
		return
	case rest == "terminals" && r.Method == http.MethodPost:
		t, err := s.orch.OpenTerminal(id)
		if err != nil {
			s.writeErr(w, err)
			return
		}
		s.writeJSON(w, http.StatusOK, map[string]string{"id": t.ID})
		return
	}

	// /projects/{id}/terminals/{tid}/…
	if tpath, ok := cutPrefix(rest, "terminals/"); ok {
		tid, action := take(tpath)
		t, err := s.orch.Terms().Get(tid)
		if err != nil {
			s.writeErr(w, err)
			return
		}
		switch {
		case action == "stream" && r.Method == http.MethodGet:
			s.handleTermStream(w, r, t)
			return
		case action == "write" && r.Method == http.MethodPost:
			var body struct {
				Data string `json:"data"`
			}
			if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 64*1024)).Decode(&body); err != nil {
				s.writeErr(w, badRequest("term write: %v", err))
				return
			}
			s.writeJSONOrErr(w, map[string]bool{"ok": true}, t.Write([]byte(body.Data)))
			return
		case action == "resize" && r.Method == http.MethodPost:
			var body struct {
				Cols uint16 `json:"cols"`
				Rows uint16 `json:"rows"`
			}
			if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 4096)).Decode(&body); err != nil {
				s.writeErr(w, badRequest("term resize: %v", err))
				return
			}
			s.writeJSONOrErr(w, map[string]bool{"ok": true}, t.Resize(body.Cols, body.Rows))
			return
		case action == "" && r.Method == http.MethodDelete:
			s.writeJSONOrErr(w, map[string]bool{"ok": true}, s.orch.Terms().Close(tid))
			return
		}
	}

	// /projects/{id}/sessions/{sid}/…
	sidPath, ok := cutPrefix(rest, "sessions/")
	if !ok {
		http.NotFound(w, r)
		return
	}
	sid, tail2 := take(sidPath)
	if sid == "" {
		http.NotFound(w, r)
		return
	}

	switch {
	case tail2 == "" && r.Method == http.MethodDelete:
		s.withAdapter(w, id, func(a agent.Adapter) {
			s.writeJSONOrErr(w, map[string]bool{"ok": true}, a.DeleteSession(r.Context(), sid))
		})
		return
	case tail2 == "" && r.Method == http.MethodPatch:
		s.withAdapter(w, id, func(a agent.Adapter) {
			var body struct {
				Title string `json:"title"`
			}
			if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 4096)).Decode(&body); err != nil {
				s.writeErr(w, badRequest("rename body: %v", err))
				return
			}
			title := strings.TrimSpace(body.Title)
			if title == "" {
				s.writeErr(w, badRequest("title is required"))
				return
			}
			s.writeJSONOrErr(w, map[string]bool{"ok": true}, a.RenameSession(r.Context(), sid, title))
		})
		return
	case tail2 == "messages" && r.Method == http.MethodGet:
		limit := 0
		if v := r.URL.Query().Get("limit"); v != "" {
			limit, _ = strconv.Atoi(v)
		}
		s.withAdapter(w, id, func(a agent.Adapter) {
			msgs, err := a.Messages(r.Context(), sid, limit)
			s.writeJSONOrErr(w, msgs, err)
		})
		return
	case tail2 == "prompt" && r.Method == http.MethodPost:
		s.withAdapter(w, id, func(a agent.Adapter) {
			var req protocol.PromptRequest
			if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<20)).Decode(&req); err != nil {
				s.writeErr(w, badRequest("prompt body: %v", err))
				return
			}
			if req.Text == "" {
				s.writeErr(w, badRequest("prompt text is required"))
				return
			}
			s.writeJSONOrErr(w, map[string]bool{"ok": true}, a.Prompt(r.Context(), sid, req))
		})
		return
	case tail2 == "abort" && r.Method == http.MethodPost:
		s.withAdapter(w, id, func(a agent.Adapter) {
			s.writeJSONOrErr(w, map[string]bool{"ok": true}, a.Abort(r.Context(), sid))
		})
		return
	case tail2 == "branch" && r.Method == http.MethodPost:
		s.withAdapter(w, id, func(a agent.Adapter) {
			var body struct {
				Branch string `json:"branch"`
			}
			if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 4096)).Decode(&body); err != nil {
				s.writeErr(w, badRequest("branch body: %v", err))
				return
			}
			if body.Branch == "" {
				s.writeErr(w, badRequest("branch is required"))
				return
			}
			s.writeJSONOrErr(w, map[string]bool{"ok": true},
				a.SetBranch(r.Context(), sid, body.Branch))
		})
		return
	}

	// /projects/{id}/vcs — git state of the project (repo badge + branch).
	if tail == "vcs" && r.Method == http.MethodGet {
		s.withAdapter(w, id, func(a agent.Adapter) {
			s.writeJSONOrErr(w, a.Vcs(r.Context()), nil)
		})
		return
	}
	// /projects/{id}/branches — branch list for the composer picker.
	if tail == "branches" && r.Method == http.MethodGet {
		s.withAdapter(w, id, func(a agent.Adapter) {
			s.writeJSONOrErr(w, a.Branches(r.Context()), nil)
		})
		return
	}

	// /projects/{id}/sessions/{sid}/permissions/{pid}
	if pid, has := cutPrefix(tail2, "permissions/"); has && pid != "" && r.Method == http.MethodPost {
		s.withAdapter(w, id, func(a agent.Adapter) {
			var body struct {
				Response string `json:"response"`
			}
			if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 4096)).Decode(&body); err != nil {
				s.writeErr(w, badRequest("permission body: %v", err))
				return
			}
			switch body.Response {
			case protocol.PermissionOnce, protocol.PermissionAlways, protocol.PermissionReject:
			default:
				s.writeErr(w, badRequest("response must be once|always|reject"))
				return
			}
			s.writeJSONOrErr(w, map[string]bool{"ok": true},
				a.ReplyPermission(r.Context(), sid, pid, body.Response))
		})
		return
	}

	// /projects/{id}/sessions/{sid}/forms/{fid} — answer a pending form
	// (question tool): body {"answer": {"<fieldKey>": "<value>"}}.
	if fid, has := cutPrefix(tail2, "forms/"); has && fid != "" && r.Method == http.MethodPost {
		s.withAdapter(w, id, func(a agent.Adapter) {
			var body struct {
				Answer map[string]any `json:"answer"`
			}
			if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 64*1024)).Decode(&body); err != nil {
				s.writeErr(w, badRequest("form body: %v", err))
				return
			}
			if len(body.Answer) == 0 {
				s.writeErr(w, badRequest("answer must not be empty"))
				return
			}
			s.writeJSONOrErr(w, map[string]bool{"ok": true},
				a.ReplyForm(r.Context(), sid, fid, body.Answer))
		})
		return
	}

	http.NotFound(w, r)
}

// take splits "a/b/c" into ("a", "b/c").
func take(path string) (head, rest string) {
	for i := 0; i < len(path); i++ {
		if path[i] == '/' {
			return path[:i], path[i+1:]
		}
	}
	return path, ""
}

// cutPrefix reports whether path starts with prefix and returns the remainder.
func cutPrefix(path, prefix string) (string, bool) {
	if len(path) >= len(prefix) && path[:len(prefix)] == prefix {
		return path[len(prefix):], true
	}
	return "", false
}

func (s *Server) withAdapter(w http.ResponseWriter, projectID string, fn func(a agent.Adapter)) {
	a, err := s.orch.AdapterOf(projectID)
	if err != nil {
		s.writeErr(w, err)
		return
	}
	fn(a)
}

func (s *Server) handleAddProject(w http.ResponseWriter, r *http.Request) {
	var body struct {
		Path     string `json:"path"`
		Mode     string `json:"mode"`
		URL      string `json:"url"`
		Provider string `json:"provider"`
	}
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 64*1024)).Decode(&body); err != nil {
		s.writeErr(w, badRequest("project body: %v", err))
		return
	}
	pv, err := s.orch.AddProject(r.Context(), body.Path, body.Mode, body.URL, body.Provider)
	s.writeJSONOrErr(w, pv, err)
}

func (s *Server) handleCreateSession(w http.ResponseWriter, r *http.Request, projectID string) {
	var body struct {
		Title  string `json:"title"`
		Branch string `json:"branch"`
	}
	// An absent body is fine (untitled session); a malformed one is ignored —
	// there is nothing in it the API requires.
	_ = json.NewDecoder(http.MaxBytesReader(w, r.Body, 64*1024)).Decode(&body)
	a, err := s.orch.AdapterOf(projectID)
	if err != nil {
		s.writeErr(w, err)
		return
	}
	sess, err := a.CreateSession(r.Context(), body.Title)
	if err == nil && body.Branch != "" && sess.ID != "" {
		// best-effort: the branch travels as a session instruction; a failed
		// pin must not fail the session itself.
		_ = a.SetBranch(r.Context(), sess.ID, body.Branch)
	}
	s.writeJSONOrErr(w, sess, err)
}

// handleTermStream streams one terminal's output as SSE: each frame is a
// base64 PTY chunk; a final `closed` frame marks the shell's exit.
func (s *Server) handleTermStream(w http.ResponseWriter, r *http.Request, t *term.Terminal) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		http.Error(w, "streaming unsupported", http.StatusInternalServerError)
		return
	}
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, ": connected\n\n")
	flusher.Flush()

	ch, unsubscribe := t.Subscribe()
	defer unsubscribe()

	for {
		select {
		case <-r.Context().Done():
			return
		case chunk, ok := <-ch:
			if !ok || chunk == nil {
				fmt.Fprint(w, "data: closed\n\n")
				flusher.Flush()
				return
			}
			fmt.Fprintf(w, "data: %s\n\n", base64.StdEncoding.EncodeToString(chunk))
			flusher.Flush()
		}
	}
}

// handleSSE streams neutral events to one webview client: replay ring first,
// then live events, with comment keep-alives every 15s.
func (s *Server) handleSSE(w http.ResponseWriter, r *http.Request) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		http.Error(w, "streaming unsupported", http.StatusInternalServerError)
		return
	}
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, ": connected\n\n")
	flusher.Flush()

	events, unsubscribe := s.orch.Subscribe()
	defer unsubscribe()

	heartbeat := time.NewTicker(15 * time.Second)
	defer heartbeat.Stop()

	for {
		select {
		case <-r.Context().Done():
			return
		case <-heartbeat.C:
			if _, err := fmt.Fprint(w, ": ping\n\n"); err != nil {
				return
			}
			flusher.Flush()
		case env := <-events:
			raw, err := json.Marshal(env)
			if err != nil {
				continue
			}
			if _, err := fmt.Fprintf(w, "data: %s\n\n", raw); err != nil {
				return
			}
			flusher.Flush()
		}
	}
}

func (s *Server) writeJSON(w http.ResponseWriter, status int, body any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(body)
}

func (s *Server) writeJSONOrErr(w http.ResponseWriter, body any, err error) {
	if err != nil {
		s.writeErr(w, err)
		return
	}
	s.writeJSON(w, http.StatusOK, body)
}

func (s *Server) writeErr(w http.ResponseWriter, err error) {
	var br *badRequestError
	if errors.As(err, &br) {
		s.writeJSON(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
		return
	}
	var nr *orchestrator.NotReadyError
	if errors.As(err, &nr) {
		w.Header().Set("Retry-After", "1")
		s.writeJSON(w, http.StatusServiceUnavailable, map[string]string{"error": err.Error()})
		return
	}
	s.writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
}

type badRequestError struct{ msg string }

func (e *badRequestError) Error() string { return e.msg }

func badRequest(format string, args ...any) error {
	return &badRequestError{msg: fmt.Sprintf(format, args...)}
}
