# Circulo — Git checkpoints

- **Status:** Capture and branch status implemented (2026-11-26). Rewind next.
- **Related:** [PRD](prd.md) · [TRD](trd.md) · [Implementation](implement.md) · [Projects](projects.md).
- **Reference:** Waku's checkpoint module (ideas only; no code copied).

## What exists

- `crates/git` captures a checkpoint of the whole worktree — tracked and
  untracked files, ignoring what `.gitignore` ignores — without touching the
  user's branch, index or working tree.
- The core captures one **start** snapshot before the provider starts a turn and
  one **end** snapshot after it. A checkpoint is a safety net: a Git failure is
  logged/ignored and never fails the turn.
- The composer footer shows the real branch when the session's project is a
  repository (`RepoStatus`); a plain folder or a projectless session hides it.
- A turn's refs are:
  `refs/circulo/session-<uuid>/turn-<n>-start` and `...-turn-<n>-end`.
  They keep the commits reachable across garbage collection.
- Rewind is not wired to the UI yet. The restore primitive exists and has
  tests.

## Capture

1. `git rev-parse --git-common-dir` finds the repository, even from a worktree.
2. A temporary index (`.circulo-checkpoint-index-<uuid>` in the common dir) is
   filled with `git read-tree HEAD` (when a HEAD exists) and `git add -A -- .`.
3. `git write-tree` and `git commit-tree` produce a commit; `git update-ref`
   points the session/turn ref at it.
4. The temporary index is removed. The user's own index is never used, so a
   staged file stays staged and an unstaged edit stays unstaged.

The author/committer identity comes from environment variables
(`Circulo <checkpoint@circulo.local>`), so no repository config is required and
nothing prompts.

## Restore (primitive, tested)

```sh
git restore --source=<commit> --worktree --staged -- .
git clean -fd -- .
git reset --quiet -- .     # only when HEAD exists
```

This is exact: tracked edits go back to the snapshot and files created after it
are removed. Ignored files are left alone. It is destructive by design, so the
rewind UI must confirm before calling it.

## The rewind plan

- **Target:** a turn's start or end snapshot. Rewinding to the **start** of turn
  N restores the files as they were before the prompt and removes turn N and
  everything after it; rewinding to the **end** of turn N keeps N and removes
  only later turns.
- **Transcript:** the core deletes the trimmed turns and parts from SQLite and
  answers with a fresh `History`, so files and transcript move together.
- **Provider context:** ACP has no rewind/fork primitive. The honest v1 drops
  the stored provider session key after a rewind, so the next prompt starts a
  fresh agent conversation over the restored files. The transcript stays; the
  agent's internal memory of the removed turns does not. This must be visible in
  the confirmation.
- **UI:** a per-turn action (hover or context) offers "Rewind to before this
  turn" / "Rewind to here". A card above the composer confirms: files restored,
  later turns removed, untracked files created after the checkpoint deleted, and
  the agent context reset. Cancel is the default.
- **Wire:** `Rewind { session, turn, to: Start | End }`; the daemon answers with
  `History` (trimmed) and `RepoStatus`.
- **Safety:** refuse while a turn is in flight; refuse when the ref is missing;
  require the project to be a repository.

## Worktrees

Waku separates checkpoints from worktrees: a checkpoint snapshots whatever
folder the session uses, and a worktree is an optional isolation mode. Circulo
does the same. Sessions currently run in the project folder; a worktree per
session can arrive later without changing the checkpoint format. That keeps the
projects model stable.

## Verification so far

- `crates/git` tests against a real temporary repository: capture restores the
  exact files, removes files created later, leaves the user's index untouched,
  and reads the current branch.
- Core tests: a repo project leaves `turn-1-start` and `turn-1-end` refs and
  reports the branch; a plain folder produces no refs and `(false, None)`.
- Manual smoke: a real repository project showed the real `main` branch in the
  footer, and a demo turn left both refs under `refs/circulo/session-<id>/`.

## Out of scope for this slice

- The rewind UI and transcript truncation (next).
- Provider-native rewind/fork (ACP does not offer one).
- Worktrees per session.
- Diff/file summaries per checkpoint; the right panel will read the refs.
- Deleting checkpoint refs when a session is deleted; there is no session delete
  yet.
