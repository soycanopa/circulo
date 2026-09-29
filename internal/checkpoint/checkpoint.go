// Package checkpoint stores per-turn workspace snapshots as git refs, so a
// turn can be rewound without asking the agent backend to cooperate.
//
// The design follows one idea: a turn is identified by the state of the
// workspace before it ran and after it ran, and both of those states already
// exist inside the repository the agent is editing. A ref is a name for a
// commit, so naming the two states is enough — no side database, no branch,
// no stash entry, and no provider support. That last part is why this is in
// Circulo at all: OpenCode can revert a turn and omp cannot, so a
// provider-specific rewind would have worked for one backend only.
//
// Two deliberate choices:
//
//   - Snapshots never touch the working tree. `git stash create` builds a
//     commit object from the dirty tree and leaves the index, the worktree
//     and refs/stash exactly as they were. An alternative that stashes and
//     pops would corrupt the user's own stash list on any crash.
//   - Rewind restores the worktree from the ref and leaves HEAD alone. Moving
//     the branch would discard commits the user made during the session, which
//     is not what "undo the agent's last turn" should mean. Because the
//     pre-rewind state is itself captured first, the action is undoable.
package checkpoint

import (
	"bytes"
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"
)

// refPrefix is deliberately outside refs/heads and refs/tags: these names are
// bookkeeping, not branches a user should be switching between.
const refPrefix = "refs/circulo"

// Kind distinguishes the two states that bound a turn.
type Kind string

const (
	// KindStart is the workspace as it was when the turn began. This is both
	// what a rewind restores and the base the turn's diff is measured from.
	KindStart Kind = "start"
	// KindEnd is the workspace as it was when the turn finished.
	KindEnd Kind = "end"
	// KindGuard is the state captured immediately before a rewind, so the
	// rewind itself can be undone.
	KindGuard Kind = "guard"
)

// Checkpoint is one stored workspace state.
type Checkpoint struct {
	SessionID string `json:"sessionID"`
	Turn      int    `json:"turn"`
	Kind      Kind   `json:"kind"`
	Ref       string `json:"ref"`
	Commit    string `json:"commit"`
	CreatedAt int64  `json:"createdAt"`
}

// ErrNotARepo means the project directory is not under git, so no checkpoint
// can exist. It is a normal condition — a project is any folder — and callers
// should treat it as "this capability is unavailable", not as a failure.
type ErrNotARepo struct{ Dir string }

func (e *ErrNotARepo) Error() string {
	return fmt.Sprintf("checkpoint: %s is not a git repository", e.Dir)
}

// Available reports whether dir can hold checkpoints at all. A repository
// with no commits cannot: there is no tree to point a ref at until the first
// commit exists.
func Available(dir string) bool {
	if _, err := run(dir, "rev-parse", "--git-dir"); err != nil {
		return false
	}
	_, err := run(dir, "rev-parse", "--verify", "HEAD")
	return err == nil
}

// Capture writes a ref for the current state of dir and returns it.
//
// A clean worktree resolves to HEAD without writing an object; a dirty one
// resolves through `git stash create`, which snapshots the dirty tree
// without altering it.
func Capture(ctx context.Context, dir, sessionID string, turn int, kind Kind) (Checkpoint, error) {
	if !Available(dir) {
		return Checkpoint{}, &ErrNotARepo{Dir: dir}
	}
	commit, err := snapshot(ctx, dir)
	if err != nil {
		return Checkpoint{}, err
	}
	ref := RefName(sessionID, turn, kind)
	if _, err := runCtx(ctx, dir, "update-ref", ref, commit); err != nil {
		return Checkpoint{}, fmt.Errorf("checkpoint: update-ref %s: %w", ref, err)
	}
	now := time.Now()
	return Checkpoint{
		SessionID: sessionID,
		Turn:      turn,
		Kind:      kind,
		Ref:       ref,
		Commit:    commit,
		CreatedAt: now.UnixMilli(),
	}, nil
}

// RefName is the deterministic ref for one (session, turn, kind). Exported so
// callers can prune by prefix without reimplementing the format.
func RefName(sessionID string, turn int, kind Kind) string {
	return fmt.Sprintf("%s/session-%s-turn-%d-%s", refPrefix, sanitize(sessionID), turn, kind)
}

// List returns every checkpoint for a session, ordered by turn then kind, so
// the newest is last and a UI can render it as a timeline.
func List(ctx context.Context, dir, sessionID string) ([]Checkpoint, error) {
	if !Available(dir) {
		return nil, &ErrNotARepo{Dir: dir}
	}
	prefix := sessionPrefix(sessionID)
	refs, err := listRefs(ctx, dir)
	if err != nil {
		return nil, err
	}
	var cps []Checkpoint
	for _, ref := range refs {
		if !strings.HasPrefix(ref.name, prefix) {
			continue
		}
		turn, kind, ok := parseRef(ref.name, prefix)
		if !ok {
			continue
		}
		cps = append(cps, Checkpoint{
			SessionID: sessionID,
			Turn:      turn,
			Kind:      kind,
			Ref:       ref.name,
			Commit:    ref.commit,
			CreatedAt: ref.createdAt,
		})
	}
	sort.Slice(cps, func(i, j int) bool {
		if cps[i].Turn != cps[j].Turn {
			return cps[i].Turn < cps[j].Turn
		}
		// Within a turn, the start is the state the turn began from and the
		// end is where it landed, so that is the order a timeline needs.
		// Sorting by the kind's name would put "end" before "start".
		return kindRank(cps[i].Kind) < kindRank(cps[j].Kind)
	})
	return cps, nil
}

func kindRank(k Kind) int {
	switch k {
	case KindStart:
		return 0
	case KindEnd:
		return 1
	case KindGuard:
		return 2
	}
	return 3
}

type refInfo struct {
	name      string
	commit    string
	createdAt int64
}

// listRefs reads every checkpoint ref. It enumerates the whole namespace and
// filters in Go rather than passing a session prefix to for-each-ref: git
// matches that argument on path components, and "…-turn-" is not a component
// of "…-turn-3-start", so a prefix there silently returns nothing.
func listRefs(ctx context.Context, dir string) ([]refInfo, error) {
	out, err := runCtx(ctx, dir, "for-each-ref",
		"--format=%(refname)%09%(objectname)%09%(creatordate:unix)", refPrefix+"/")
	if err != nil {
		return nil, fmt.Errorf("checkpoint: for-each-ref: %w", err)
	}
	var refs []refInfo
	for _, line := range strings.Split(out, "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		parts := strings.Split(line, "\t")
		if len(parts) < 3 {
			continue
		}
		secs, _ := strconv.ParseInt(parts[2], 10, 64)
		refs = append(refs, refInfo{name: parts[0], commit: parts[1], createdAt: secs * 1000})
	}
	return refs, nil
}

// Restore puts dir's working tree back to the state stored in ref. HEAD and the
// current branch are not moved: a rewind that discarded the user's own commits
// would be a different, much more dangerous operation than the one asked for.
//
// Files created after the checkpoint are removed, because restoring a tree
// means restoring a tree. The caller must have confirmed the action.
func Restore(ctx context.Context, dir, ref string) error {
	if !Available(dir) {
		return &ErrNotARepo{Dir: dir}
	}
	// A ref under our own prefix is the only thing Restore will act on. The
	// ref reaches us from the webview, so this is a boundary, not a nicety.
	if !strings.HasPrefix(ref, refPrefix+"/") {
		return fmt.Errorf("checkpoint: refusing to restore %q: not a checkpoint ref", ref)
	}
	if _, err := runCtx(ctx, dir, "rev-parse", "--verify", ref+"^{commit}"); err != nil {
		return fmt.Errorf("checkpoint: %s is not a stored checkpoint: %w", ref, err)
	}
	// read-tree --reset -u rewrites the index and the worktree to the ref's
	// tree. It does not touch HEAD, and it removes files the checkpoint did
	// not have — which is the honest meaning of "undo this turn".
	if _, err := runCtx(ctx, dir, "read-tree", "--reset", "-u", ref); err != nil {
		return fmt.Errorf("checkpoint: restore %s: %w", ref, err)
	}
	return nil
}

// Diff returns the unified diff between two stored checkpoints, which is what
// the rewind confirmation shows before anything is destroyed.
func Diff(ctx context.Context, dir, fromRef, toRef string) (string, error) {
	if !Available(dir) {
		return "", &ErrNotARepo{Dir: dir}
	}
	return runCtx(ctx, dir, "diff", "--stat", "--patch", fromRef, toRef)
}

// snapshot resolves the current worktree state to a commit, without touching
// the user's index, worktree or stash list.
//
// The obvious tool is wrong here. `git stash create` leaves everything alone,
// but it omits untracked files — and the files an agent creates are the whole
// point of a checkpoint. `git add -A` would capture them but rewrites the
// user's index. So the work is done in an alternate index: build a tree there,
// and commit it without ever looking at the real one.
func snapshot(ctx context.Context, dir string) (string, error) {
	tmp, err := os.MkdirTemp("", "circulo-checkpoint-")
	if err != nil {
		return "", fmt.Errorf("checkpoint: temp index: %w", err)
	}
	defer os.RemoveAll(tmp)
	idx := filepath.Join(tmp, "index")
	env := append(os.Environ(), "GIT_INDEX_FILE="+idx)

	// Seed the alternate index from HEAD, or start empty in a repo that has
	// no commits yet.
	if _, err := runEnv(ctx, dir, env, "read-tree", "HEAD"); err != nil {
		if _, err := runEnv(ctx, dir, env, "read-tree", "--empty"); err != nil {
			return "", fmt.Errorf("checkpoint: seed index: %w", err)
		}
	}
	// -A stages deletions and new files alike. .gitignore still applies, which
	// is right: build artifacts are not part of the turn.
	if _, err := runEnv(ctx, dir, env, "add", "-A", "--", "."); err != nil {
		return "", fmt.Errorf("checkpoint: stage: %w", err)
	}
	tree, err := runEnv(ctx, dir, env, "write-tree")
	if err != nil {
		return "", fmt.Errorf("checkpoint: write-tree: %w", err)
	}
	// If nothing changed, the snapshot is HEAD. Reusing it keeps repeated
	// checkpoints of an idle workspace identical, so "did this turn change
	// anything" is a comparison of SHAs rather than of file diffs.
	if head, err := runCtx(ctx, dir, "rev-parse", "--verify", "HEAD"); err == nil {
		if headTree, err := runCtx(ctx, dir, "rev-parse", "HEAD^{tree}"); err == nil && headTree == tree {
			return strings.TrimSpace(head), nil
		}
	}
	args := []string{"commit-tree", tree}
	if _, err := runCtx(ctx, dir, "rev-parse", "--verify", "HEAD"); err == nil {
		args = append(args, "-p", "HEAD")
	}
	commit, err := runStdin(ctx, dir, "circulo checkpoint\n", args...)
	if err != nil {
		return "", fmt.Errorf("checkpoint: commit-tree: %w", err)
	}
	return commit, nil
}

// Dirty reports whether the worktree differs from HEAD, so the UI can warn
// that a rewind would discard work in progress.
func Dirty(ctx context.Context, dir string) (bool, error) {
	if !Available(dir) {
		return false, &ErrNotARepo{Dir: dir}
	}
	out, err := runCtx(ctx, dir, "status", "--porcelain")
	if err != nil {
		return false, fmt.Errorf("checkpoint: status: %w", err)
	}
	return strings.TrimSpace(out) != "", nil
}

// Prune deletes checkpoints older than keep turns for a session, so a long
// project does not accumulate refs forever.
func Prune(ctx context.Context, dir, sessionID string, keep int) error {
	if !Available(dir) {
		return &ErrNotARepo{Dir: dir}
	}
	prefix := sessionPrefix(sessionID)
	refs, err := listRefs(ctx, dir)
	if err != nil {
		return err
	}
	byTurn := map[int][]string{}
	var turns []int
	for _, ref := range refs {
		if !strings.HasPrefix(ref.name, prefix) {
			continue
		}
		turn, _, ok := parseRef(ref.name, prefix)
		if !ok {
			continue
		}
		if _, seen := byTurn[turn]; !seen {
			turns = append(turns, turn)
		}
		byTurn[turn] = append(byTurn[turn], ref.name)
	}
	sort.Ints(turns)
	if keep < 0 {
		keep = 0
	}
	for i, turn := range turns {
		if i >= len(turns)-keep {
			continue
		}
		for _, ref := range byTurn[turn] {
			if _, err := runCtx(ctx, dir, "update-ref", "-d", ref); err != nil {
				return fmt.Errorf("checkpoint: delete %s: %w", ref, err)
			}
		}
	}
	return nil
}

func sessionPrefix(sessionID string) string {
	return fmt.Sprintf("%s/session-%s-turn-", refPrefix, sanitize(sessionID))
}

func parseRef(ref, prefix string) (int, Kind, bool) {
	rest, ok := strings.CutPrefix(ref, prefix)
	if !ok {
		return 0, "", false
	}
	turnStr, kind, ok := strings.Cut(rest, "-")
	if !ok {
		return 0, "", false
	}
	n, err := strconv.Atoi(turnStr)
	if err != nil {
		return 0, "", false
	}
	switch Kind(kind) {
	case KindStart, KindEnd, KindGuard:
		return n, Kind(kind), true
	}
	return 0, "", false
}

// sanitize keeps a session id from escaping the ref namespace or producing a
// ref git will reject. Ids come from the agent backends, so they are not
// trusted to be ref-safe.
func sanitize(s string) string {
	var b strings.Builder
	for _, r := range s {
		switch {
		case r >= 'a' && r <= 'z', r >= 'A' && r <= 'Z', r >= '0' && r <= '9', r == '-', r == '_':
			b.WriteRune(r)
		default:
			b.WriteRune('-')
		}
	}
	out := b.String()
	if out == "" {
		return "unknown"
	}
	return out
}

func run(dir string, args ...string) (string, error) {
	return runCtx(context.Background(), dir, args...)
}

func runCtx(ctx context.Context, dir string, args ...string) (string, error) {
	return runFull(ctx, dir, os.Environ(), "", args...)
}

func runEnv(ctx context.Context, dir string, env []string, args ...string) (string, error) {
	return runFull(ctx, dir, env, "", args...)
}

func runStdin(ctx context.Context, dir, stdin string, args ...string) (string, error) {
	return runFull(ctx, dir, os.Environ(), stdin, args...)
}

func runFull(ctx context.Context, dir string, env []string, stdin string, args ...string) (string, error) {
	cmd := exec.CommandContext(ctx, "git", args...)
	cmd.Dir = dir
	cmd.Env = env
	if stdin != "" {
		cmd.Stdin = strings.NewReader(stdin)
	}
	var out, errOut bytes.Buffer
	cmd.Stdout = &out
	cmd.Stderr = &errOut
	if err := cmd.Run(); err != nil {
		detail := strings.TrimSpace(errOut.String())
		if detail == "" {
			detail = err.Error()
		}
		return "", fmt.Errorf("git %s: %s", strings.Join(args, " "), detail)
	}
	return strings.TrimSpace(out.String()), nil
}
