package checkpoint

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// repo makes a real git repository with one commit. Real git, not a fake: the
// whole package is git plumbing, and a mock would only prove the mock works.
func repo(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	git(t, dir, "init", "-b", "main")
	git(t, dir, "config", "user.email", "t@example.com")
	git(t, dir, "config", "user.name", "test")
	write(t, dir, "README.md", "one\n")
	git(t, dir, "add", ".")
	git(t, dir, "commit", "-m", "initial")
	return dir
}

func git(t *testing.T, dir string, args ...string) string {
	t.Helper()
	cmd := exec.Command("git", args...)
	cmd.Dir = dir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("git %s: %v\n%s", strings.Join(args, " "), err, out)
	}
	return strings.TrimSpace(string(out))
}

func write(t *testing.T, dir, name, body string) {
	t.Helper()
	p := filepath.Join(dir, name)
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(p, []byte(body), 0o644); err != nil {
		t.Fatal(err)
	}
}

func read(t *testing.T, dir, name string) string {
	t.Helper()
	b, err := os.ReadFile(filepath.Join(dir, name))
	if err != nil {
		return ""
	}
	return string(b)
}

func TestAvailable(t *testing.T) {
	dir := repo(t)
	if !Available(dir) {
		t.Error("a real repo should be available")
	}
	plain := t.TempDir()
	if Available(plain) {
		t.Error("a plain directory must not report as a repo")
	}
}

// A repository with no commits has no tree to name, so checkpoints must be
// unavailable rather than capturing something meaningless.
func TestAvailable_FalseWithoutCommits(t *testing.T) {
	dir := t.TempDir()
	git(t, dir, "init", "-b", "main")
	if Available(dir) {
		t.Error("a repo with no commits cannot hold checkpoints")
	}
	if _, err := Capture(context.Background(), dir, "s1", 1, KindStart); err == nil {
		t.Error("expected an error capturing in a repo with no commits")
	}
}

func TestCapture_CleanTreeResolvesToHead(t *testing.T) {
	dir := repo(t)
	head := git(t, dir, "rev-parse", "HEAD")
	cp, err := Capture(context.Background(), dir, "ses_1", 1, KindStart)
	if err != nil {
		t.Fatal(err)
	}
	if cp.Commit != head {
		t.Errorf("commit = %s, want HEAD %s", cp.Commit, head)
	}
	if got := git(t, dir, "rev-parse", RefName("ses_1", 1, KindStart)); got != head {
		t.Errorf("ref points at %s, want %s", got, head)
	}
}

// The critical property: capturing a dirty tree must snapshot it without
// disturbing the user's working tree, index or stash list.
func TestCapture_DirtyTreeDoesNotDisturbWorktree(t *testing.T) {
	dir := repo(t)
	write(t, dir, "README.md", "modified by agent\n")
	unstagedBefore, _ := exec.Command("git", "-C", dir, "status", "--porcelain").Output()
	stashBefore := git(t, dir, "stash", "list")

	cp, err := Capture(context.Background(), dir, "ses_1", 1, KindStart)
	if err != nil {
		t.Fatal(err)
	}
	if got := read(t, dir, "README.md"); got != "modified by agent\n" {
		t.Errorf("capture changed the worktree: %q", got)
	}
	unstagedAfter, _ := exec.Command("git", "-C", dir, "status", "--porcelain").Output()
	if string(unstagedBefore) != string(unstagedAfter) {
		t.Errorf("capture changed git status:\nbefore %q\nafter  %q", unstagedBefore, unstagedAfter)
	}
	if got := git(t, dir, "stash", "list"); got != stashBefore {
		t.Errorf("capture touched the stash list: before %q after %q", stashBefore, got)
	}
	// ...and the ref really does hold the dirty content.
	blob := git(t, dir, "show", cp.Ref+":README.md")
	if blob != "modified by agent" {
		t.Errorf("ref content = %q, want the dirty state", blob)
	}
}

func TestCapture_IncludesUntrackedFiles(t *testing.T) {
	dir := repo(t)
	write(t, dir, "new-file.txt", "created by agent\n")
	cp, err := Capture(context.Background(), dir, "ses_1", 2, KindStart)
	if err != nil {
		t.Fatal(err)
	}
	if got := git(t, dir, "show", cp.Ref+":new-file.txt"); got != "created by agent" {
		t.Errorf("untracked file missing from checkpoint: %q", got)
	}
}

func TestCapture_RefsAreOutsideBranchesAndTags(t *testing.T) {
	dir := repo(t)
	if _, err := Capture(context.Background(), dir, "ses_1", 1, KindStart); err != nil {
		t.Fatal(err)
	}
	branches := git(t, dir, "branch", "--list")
	if strings.Contains(branches, "circulo") {
		t.Errorf("checkpoint leaked into a branch: %q", branches)
	}
	refs := git(t, dir, "for-each-ref", "--format=%(refname)", refPrefix+"/")
	if !strings.Contains(refs, "refs/circulo/") {
		t.Errorf("ref not stored under %s: %q", refPrefix, refs)
	}
}

func TestList_OrderedByTurnThenKind(t *testing.T) {
	dir := repo(t)
	ctx := context.Background()
	for _, turn := range []int{2, 1} {
		for _, k := range []Kind{KindEnd, KindStart} {
			if _, err := Capture(ctx, dir, "ses_1", turn, k); err != nil {
				t.Fatal(err)
			}
		}
	}
	cps, err := List(ctx, dir, "ses_1")
	if err != nil {
		t.Fatal(err)
	}
	if len(cps) != 4 {
		t.Fatalf("got %d checkpoints, want 4", len(cps))
	}
	want := []struct {
		turn int
		kind Kind
	}{{1, KindStart}, {1, KindEnd}, {2, KindStart}, {2, KindEnd}}
	for i, w := range want {
		if cps[i].Turn != w.turn || cps[i].Kind != w.kind {
			t.Errorf("checkpoint %d = turn %d/%s, want %d/%s", i, cps[i].Turn, cps[i].Kind, w.turn, w.kind)
		}
	}
}

func TestList_EmptyForUnknownSession(t *testing.T) {
	dir := repo(t)
	cps, err := List(context.Background(), dir, "nobody")
	if err != nil {
		t.Fatal(err)
	}
	if len(cps) != 0 {
		t.Errorf("got %d, want none", len(cps))
	}
}

func TestRestore_WorktreeOnlyHeadStays(t *testing.T) {
	dir := repo(t)
	ctx := context.Background()
	if _, err := Capture(ctx, dir, "ses_1", 1, KindStart); err != nil {
		t.Fatal(err)
	}
	// The turn edits and commits.
	write(t, dir, "README.md", "agent rewrote this\n")
	write(t, dir, "added.txt", "new\n")
	git(t, dir, "add", ".")
	git(t, dir, "commit", "-m", "agent work")
	headBefore := git(t, dir, "rev-parse", "HEAD")
	branchBefore := git(t, dir, "rev-parse", "--abbrev-ref", "HEAD")

	if err := Restore(ctx, dir, RefName("ses_1", 1, KindStart)); err != nil {
		t.Fatal(err)
	}
	// Worktree is back...
	if got := read(t, dir, "README.md"); got != "one\n" {
		t.Errorf("README = %q, want the pre-turn content", got)
	}
	// ...the file the turn created is gone, because restoring a tree means it...
	if fileExists(filepath.Join(dir, "added.txt")) {
		t.Error("file created by the turn survived a restore")
	}
	// ...but HEAD and the branch did not move. The user's commit is intact.
	if got := git(t, dir, "rev-parse", "HEAD"); got != headBefore {
		t.Errorf("HEAD moved to %s, want %s — a rewind must not rewrite history", got, headBefore)
	}
	if got := git(t, dir, "rev-parse", "--abbrev-ref", "HEAD"); got != branchBefore {
		t.Errorf("branch changed to %q", got)
	}
	// The agent's work is not lost either; it is now uncommitted.
	if !strings.Contains(git(t, dir, "status", "--porcelain"), "README.md") {
		t.Error("expected the rewound work to still be present as uncommitted changes")
	}
}

// The ref arrives from the webview, so Restore must refuse anything outside its
// own namespace.
func TestRestore_RefusesForeignRef(t *testing.T) {
	dir := repo(t)
	head := git(t, dir, "rev-parse", "HEAD")
	if err := Restore(context.Background(), dir, head); err == nil {
		t.Fatal("expected a refusal for a non-checkpoint ref")
	}
	if err := Restore(context.Background(), dir, "refs/heads/main"); err == nil {
		t.Fatal("expected a refusal for a branch ref")
	}
	if err := Restore(context.Background(), dir, "../../etc"); err == nil {
		t.Fatal("expected a refusal for a path-like ref")
	}
}

func TestRestore_UnknownCheckpoint(t *testing.T) {
	dir := repo(t)
	err := Restore(context.Background(), dir, RefName("ses_1", 99, KindStart))
	if err == nil {
		t.Fatal("expected an error for a checkpoint that was never stored")
	}
}

func TestDiff_ShowsWhatRewindWouldDiscard(t *testing.T) {
	dir := repo(t)
	ctx := context.Background()
	start, err := Capture(ctx, dir, "ses_1", 1, KindStart)
	if err != nil {
		t.Fatal(err)
	}
	write(t, dir, "README.md", "agent changed this\n")
	end, err := Capture(ctx, dir, "ses_1", 1, KindEnd)
	if err != nil {
		t.Fatal(err)
	}
	d, err := Diff(ctx, dir, start.Ref, end.Ref)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(d, "agent changed this") || !strings.Contains(d, "README.md") {
		t.Errorf("diff does not describe the change:\n%s", d)
	}
}

func TestDirty(t *testing.T) {
	dir := repo(t)
	ctx := context.Background()
	d, err := Dirty(ctx, dir)
	if err != nil {
		t.Fatal(err)
	}
	if d {
		t.Error("fresh repo should be clean")
	}
	write(t, dir, "README.md", "dirty\n")
	d, err = Dirty(ctx, dir)
	if err != nil {
		t.Fatal(err)
	}
	if !d {
		t.Error("modified worktree should report dirty")
	}
}

func TestPrune_KeepsNewestTurns(t *testing.T) {
	dir := repo(t)
	ctx := context.Background()
	for turn := 1; turn <= 5; turn++ {
		for _, k := range []Kind{KindStart, KindEnd} {
			if _, err := Capture(ctx, dir, "ses_1", turn, k); err != nil {
				t.Fatal(err)
			}
		}
	}
	if err := Prune(ctx, dir, "ses_1", 2); err != nil {
		t.Fatal(err)
	}
	cps, err := List(ctx, dir, "ses_1")
	if err != nil {
		t.Fatal(err)
	}
	if len(cps) != 4 {
		t.Fatalf("got %d checkpoints, want 4 (2 turns kept)", len(cps))
	}
	if cps[0].Turn != 4 || cps[len(cps)-1].Turn != 5 {
		t.Errorf("kept turns %d..%d, want 4..5", cps[0].Turn, cps[len(cps)-1].Turn)
	}
}

func TestPrune_LeavesOtherSessionsAlone(t *testing.T) {
	dir := repo(t)
	ctx := context.Background()
	for turn := 1; turn <= 3; turn++ {
		for _, k := range []Kind{KindStart, KindEnd} {
			if _, err := Capture(ctx, dir, "ses_a", turn, k); err != nil {
				t.Fatal(err)
			}
			if _, err := Capture(ctx, dir, "ses_b", turn, k); err != nil {
				t.Fatal(err)
			}
		}
	}
	if err := Prune(ctx, dir, "ses_a", 1); err != nil {
		t.Fatal(err)
	}
	b, err := List(ctx, dir, "ses_b")
	if err != nil {
		t.Fatal(err)
	}
	if len(b) != 6 {
		t.Errorf("pruning ses_a removed %d of ses_b's checkpoints", 6-len(b))
	}
}

// Session ids come from the agent backends, so an id with ref-hostile
// characters must not be able to escape the namespace.
func TestRefName_SanitizesSessionID(t *testing.T) {
	dir := repo(t)
	hostile := "ses/../../evil id"
	cp, err := Capture(context.Background(), dir, hostile, 1, KindStart)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasPrefix(cp.Ref, refPrefix+"/session-") {
		t.Errorf("ref escaped the namespace: %s", cp.Ref)
	}
	if strings.Contains(cp.Ref, "..") {
		t.Errorf("ref contains traversal: %s", cp.Ref)
	}
	// The ref still round-trips through List.
	cps, err := List(context.Background(), dir, hostile)
	if err != nil {
		t.Fatal(err)
	}
	if len(cps) != 1 {
		t.Errorf("List found %d checkpoints for a sanitized id, want 1", len(cps))
	}
}

func TestErrNotARepo(t *testing.T) {
	plain := t.TempDir()
	err := &ErrNotARepo{Dir: plain}
	if !strings.Contains(err.Error(), plain) {
		t.Errorf("error should name the directory: %v", err)
	}
}

func fileExists(p string) bool {
	_, err := os.Stat(p)
	return err == nil
}
