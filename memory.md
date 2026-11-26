# memory.md — where Circulo is and what comes next

Working notes for the next conversation. Read this first, then `AGENTS.md` and `docs/`.

Last updated: 2026-11-26, on `fix/icon-line` after PR #52; 134
tests green; CI covers fmt + clippy + test on macOS. The model-call tests in
`tests/real.rs` need OpenCode credit, which this account has none of right now
(2026-11-23): they fail with "Insufficient balance". The no-model-call load
probe (`cargo test -p circulo-provider-opencode --lib -- --ignored`) passes
against 2.3.2.

## 1. What works today

The app is usable end to end against a real agent: type a prompt, watch the turn,
answer its permission and question cards, queue or steer a follow-up, stop the
turn, quit, and find the session again.

**Window** (Rust + GPUI, `crates/ui`): the reference shell — sidebar, transcript,
composer, right-panel toggle. Tokens and geometry measured from Waku 0.1.20 on
this machine (`docs/ui.md`). Icons are Kubai Icons (MIT,
`crates/ui/assets/icons/LICENSE`) in the line style: 24 grid, 2px stroke,
`currentColor`, exact requested size. Static SVGs; GPUI does not run their
motion.

**Agent**: OpenCode over Agent Client Protocol (`opencode acp`, stdio JSON-RPC),
verified against **OpenCode 2.3.2**. Reasoning, tool calls and text stream into
the window. The demo provider (`crates/providers/demo`) still works with no auth
and exercises the shapes.

**Transcript** (`transcript.rs`, `markdown.rs`):
- GPUI `List` of turn rows, bottom-anchored: it follows the stream, and scrolling
  up shows a jump-to-latest button (`docs/ui.md` has the rules).
- Markdown: paragraphs, headings, bullet/ordered/nested lists, quotes, fenced
  code, rules, **tables** (weighted columns, delimiter alignment), **footnotes**
  (superscript refs, numbered list at the end, dangling refs stay literal), bold,
  italic, inline code, strikethrough, links (coloured, **not clickable**).
  Parse is cached per segment (`markdown::Cached`).
- Tool rows carry the provider's call id (`Part::Tool.id`), so an update finds
  its row even when another tool started in between.
- Each turn shows its own prompt above the answer.

**Cards** (`permission.rs`, `question.rs`, spec in `docs/permissions.md`):
- Permission: ACP `session/request_permission` → card above the composer with the
  agent's own options; allow kinds primary, reject kinds quiet; Enter allows,
  Escape rejects; the turn blocks until answered; a client that leaves cancels.
- Question: ACP `elicitation/create` (form) → choice rows plus free-text fields
  using the composer's input; Submit waits for required fields; Decline and
  Cancel both exist; the first text field takes the keyboard.

**Composer**: typeable (`crates/ui/src/input.rs`), Enter sends, Shift+Enter breaks
the line, block caret, grows to four lines then scrolls. No selection, undo, IME
or paste yet.

**Pickers**: model, effort and mode draw the daemon's `Options` (neutral
`ConfigOption`/`ConfigChoice`; `SetOption` back). Menus are compact popups
(360 × 320, scrolling): the model menu has two tabs — **Favorites** (star
icon) and the agent's name from the handshake — a star per row stores the
choice through the settings wire, and the agent's list is grouped under a
heading per provider (`ConfigChoice.provider`, filled by the driver, which
also strips the vendor tag from names: `deepseek/DeepSeek V4 Pro` reads
`DeepSeek V4 Pro` under a `DeepSeek` heading). Click opens the choices with
the current one marked, Escape closes, choosing returns the refreshed list.
Choices survive sessions: the driver re-applies them whenever it attaches,
and a switch made mid-turn lands at the turn's end (the picker sees it at
once, the session reads it next turn). Access stays a mock label.

**Settings**: the `settings` table (SQLite schema 3) stores the client's own
key/value JSON — `favorites.model` is a list of choice ids. The wire carries
`GetSettings`, `SetSetting` and a `Settings` message with the whole map; the
shell loads favourites at startup and redraws from the daemon's answer, never
from a local hope. A memory-only daemon refuses to store with an error.

**Continuity** (`crates/providers/opencode`, spec in `docs/providers/opencode.md`):
one `opencode acp` child per Circulo session — the agent remembers across
turns. The ACP session id is the key the driver hands back per turn; the
core stores it in `sessions.provider_session` (SQLite schema v2) and hands
it back on the next turn. A restarted daemon resumes through ACP
`session/load` (probed: the response has `configOptions`, no `sessionId`;
the replay notifications are skipped — the transcript already lives in the
database). A key the agent lost starts fresh and heals the store; a dead
child is dropped and the next turn resumes the same way. The daemon now
winds down on stdin EOF (`serve_until`): cards cancelled, agents released,
no orphans — the supervisor closes stdin and waits 2 s before escalating
to kill. Verified without model calls: `tests/continuity.rs` (scripted
agent counting prompts per process, sessions kept in a directory across
processes) and the real load probe.

**Abort**: a running turn can be stopped. The composer's send circle becomes a
stop square while `Status.running`, Escape stops too (menus and cards get the
key first). The daemon routes `CancelTurn` to the core, which closes exactly
that turn's cards — each broker entry remembers its session and turn — and asks
the driver to `cancel(session)`. OpenCode keeps a second handle to the child's
stdin (mutex + ACP session id) because the turn owns the slot, and sends ACP
`session/cancel`; `stopReason: "cancelled"` becomes `DriverError::cancelled`,
and the core reports `FinishReason::Aborted` with no error banner (the divider
reads `Stopped`). Parts already streamed stay; the child stays alive for the
next turn. `TurnFinished`
drops any card its turn left open. Verified without model calls:
`tests/permission.rs` (`FAKE_ACP_MODE=cancel`), a core test with a blocking
driver, and a daemon/client round trip.

**Queue and steering**: the core owns one FIFO queue per session. `Prompt` while
a turn runs waits; a worker drains the queue one turn at a time and emits
`TurnStarted` only when a prompt really starts, so the transcript never opens an
empty row. `TurnStarted` now carries the prompt (wire version 3), which is what
lets a queued turn show what it asked. `Steer` puts the text at the front and
cancels the running turn; ACP has no steer, so the agent ends the turn as
`cancelled` and the steer text is the next `session/prompt` (context preserved).
The queue travels whole on the `Queue` message after every change; the window
draws it above the cards and lets a row be removed, and `RemoveQueued` is a
no-op if the item already started. A closing connection clears its queues and
aborts its turn, so nothing runs for a client that is gone. Queued prompts live
in memory only: a daemon restart drops them, and a prompt that never started is
not in the transcript. Verified in core (FIFO order, remove, clear, steer) and
through the daemon (`Prompt`/`Steer`/`Queue`/`CancelTurn`).

**Projects**: a project is a canonical folder; a session is located in one. The
composer's project chip (not a sidebar section) opens the list, `New project…`
(native picker) and `No project`. Choosing a folder starts or reuses the blank
session for that folder, carrying the draft; a started session is never
repointed. `New Task` stays in the current project and reuses its blank task.
The daemon canonicalizes the path and `AddProject` answers `ProjectAdded` with
the canonical path before the refreshed `Projects` list. `SessionRef.cwd` gives
OpenCode a child per session in its own folder; projectless sessions use the
daemon folder. The sidebar row keeps the reference shape: title, project with a
folder icon, and the trailing time (`Working Ns` for the open running turn,
otherwise relative time). Schema v4 adds `projects` and backfills from
sessions. The table is the picker's list; removing an entry leaves the folder
and its sessions.

**Git checkpoints** (`crates/git`, schema-free): every turn captures a snapshot
of the project worktree before the provider starts and after it returns, stored
as a dangling commit under
`refs/circulo/session-<uuid>/turn-<n>-start|end`. The capture uses a temporary
index (`GIT_INDEX_FILE`), so the user's branch, index and working tree are never
touched; ignored files stay out. A finished turn's divider offers **Rewind** on
hover; the confirmation card picks the start snapshot (Before this turn, the turn
goes too) or the end snapshot (After this turn, only later turns go), restores
the worktree, trims turns and parts in one transaction, clears the title when
nothing remains, drops the provider key and resets the driver's child, so the
next prompt starts a fresh agent conversation over the restored files. A Git
failure never fails a turn. `RepoStatus` gives the window the real branch, or
tells it to hide the chip for a plain folder.

**Storage** (`crates/persistence`, rusqlite bundled): sessions, turns and parts in
`~/.circulo/circulo.db`; `CIRCULO_HOME` overrides the directory. The core writes
on every turn; the app reopens the newest session and hydrates its history.
The sessions table also carries the provider's session key (schema v2), which
is what makes a restart a continuation.

**Sidebar**: real sessions from the database, grouped Today / Yesterday / This
Month / month name, title from the first prompt, project folder, relative time,
the open session marked; a row opens its history, `New Task` starts a session.

**Daemon/client lifecycle**: the app spawns the daemon and holds its stdin open;
the daemon exits on EOF. The connection splits into reader + writer threads on
both sides, which is what lets an answer reach the daemon while a turn blocks.

## 2. Run it and verify it

```sh
cargo run -p circulo                                            # demo provider, no auth
CIRCULO_PROVIDER=opencode CIRCULO_PROJECT="$PWD" cargo run -p circulo
CIRCULO_HOME=/tmp/circulo-test … cargo run -p circulo           # a throwaway database
CIRCULO_PROMPT="…" … cargo run -p circulo                       # seeds the composer draft
cargo run -p circulo-daemon                                     # daemon alone
```

Checks: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets`,
`cargo test --workspace`. Real-provider tests (spend model calls, need
credit): `cargo test -p circulo-provider-opencode --test real -- --ignored`.
No-credit verification: the continuity suite (`tests/continuity.rs`) and the
load probe (`cargo test -p circulo-provider-opencode --lib -- --ignored`).

**Watching the UI without a human**: `screencapture -x -R<x>,<y>,<w>,<h>` with the
window bounds from
`osascript -e 'tell application "System Events" to tell process "circulo" to get {position, size} of window 1'`.
Keystrokes go through `System Events … keystroke "…"`; clicks and wheel events
through `osascript -l JavaScript` + `ObjC.import("CoreGraphics")`
(`CGEventCreateMouseEvent` / `CGEventCreateScrollWheelEvent2`). The first click
after another app takes focus only activates the window; click twice.

**Providers that need a scripted counterpart**: OpenCode 2.3.2 asks for
permission on `write`/`bash` but never elicits on its own, so
`crates/providers/opencode/tests/permission.rs` runs a Python ACP agent
(`FAKE_ACP_MODE=permission|question`), and `tests/continuity.rs` runs one
that counts prompts per process and persists sessions to a directory. For
the app, `CIRCULO_OPENCODE_BIN` points the daemon at that script.

**OpenCode facts worth remembering**: it auto-runs reads and searches; a
project-level `permission` config does not change ACP behaviour; the ACP session
advertises `loadSession: true` (unused so far).

## 3. Next task: a second provider

Rewind landed (2026-11-26): a finished turn's divider offers Rewind; the card
picks `Before this turn` (start snapshot, turn removed) or `After this turn` (end
snapshot, later turns removed); the core refuses while a turn runs, clears the
queue, restores the worktree, trims turns/parts in one transaction, drops the
provider key, resets the driver's child and answers with the trimmed history.
Checkpoint capture and the real branch landed just before; projects, queue/steer
and abort before that.

Session continuity landed (2026-11-23): one agent process per Circulo session,
the ACP session id in SQLite, `session/load` on restart, mid-turn switches
queued to the turn's end, clean daemon shutdown. What the model-call tests would
still prove, once there is credit: a real two-turn conversation sharing context
(`a_real_session_remembers_across_turns`) and a real restart resuming
(`a_real_session_resumes_after_a_restart`). Everything else is proven against
the scripted agent and the real load probe.

**Why a second provider next:** the Driver trait has only been proven by OpenCode
(and the demo). The PRD's v1.0 bar is at least OpenCode plus Claude Code or
Codex; a second adapter is what proves the boundary is real, not shaped around
one agent.

**Plan:**
1. Pick one: Claude Code or Codex; read its official protocol/docs first, no API
   guessing.
2. New crate under `crates/providers/`, implementing `Driver`, `Asker`,
   `cancel`, `reset`, options and continuity with the provider's own session
   key.
3. Fixtures or a scripted counterpart for the protocol shapes, the way
   OpenCode's tests do; real-CLI tests behind `--ignored`.
4. Prove: a turn streams, a permission/question card works, a model/effort
   option applies, continuity survives a daemon restart, rewind drops the
   provider session.
5. The daemon picks it with `CIRCULO_PROVIDER`; no core or UI rewrite.

## 4. After that, in plan order

1. **A second provider** (Claude Code or Codex), above.
2. **Clickable links and math** in markdown. Links need GPUI text hit-testing,
   which 0.2.2 does not expose: either keep the text layout plus element bounds,
   or build an inline layout from word-sized elements.
3. **Attachments blob store**, the piece `docs/implement.md` Phase 1 still
   leaves open inside persistence.
4. **Remote/pairing** (`docs/remote.md`): QR with a saved tailcat key + `--allow`;
   the desktop owns the tunnel. `tailcat` is not installed here. Open question:
   what the QR payload carries.

## 5. Architecture map

| Crate | Role |
|---|---|
| `crates/protocol` | Wire contract, version 3: `Hello`, `ClientRequest` (+`SetOption`, `CancelTurn`, `Steer`, `RemoveQueued`, `Rewind`, `ListProjects`, `AddProject`, `RemoveProject`), `ServerMessage` (+`Options`, `Settings`, `Queue`, `Projects`, `ProjectAdded`, `RepoStatus`), `TurnStarted` with the prompt, `Part`, `PermissionRequest`, `Question`, `ConfigOption`, `QueuedPrompt`, `ProjectSummary`, `SessionSummary`, `HistoryTurn`. One JSON per WebSocket text frame; unknown types skipped. |
| `crates/core` | Sessions, turn orchestration, one FIFO queue per session drained by a worker, the permission/question broker (entries owned by session + turn), the project list (canonical paths), Git start/end checkpoints around each turn, rewind (restore + trim + driver reset), translation between storage and wire, carrying provider session keys both ways, abort and steer routing. Speaks only the Driver trait. |
| `crates/persistence` | SQLite: sessions (+ provider key, project path), turns, parts, settings, projects; schema v4. `trim_turns` removes a suffix in one transaction. Owned by the core; nobody else opens it. |
| `crates/git` | Whole-worktree snapshots with a temporary index, private `refs/circulo/...` refs, exact restore, and the current branch. Owned by the core. |
| `crates/daemon` | Loopback WebSocket, token handshake, reader + writer per connection, provider chosen by `CIRCULO_PROVIDER`, database from `CIRCULO_HOME`, one queue worker per session, project/repo status pushes, clear-and-abort on connection close, clean shutdown on stdin EOF. |
| `crates/client` | Handshake, blocking read loop, a splittable writer half, `prompt`/`steer`/`remove_queued`/`cancel_turn`. No GPUI. |
| `crates/providers/driver` | `Driver { name, start_turn(SessionRef, prompt, ask, emit) -> Option<key>, cancel(session), reset(session), options(session), set_option(session, id, value), shutdown() }`, `SessionRef { id, resume, cwd }`, and `Asker { permission, question }`. `DriverError` carries whether the turn was cancelled, so the core can report `Aborted` instead of `Failed`. |
| `crates/providers/demo` | Canned turn, no auth. |
| `crates/providers/opencode` | ACP over stdio; one agent process per Circulo session, spawned in the session's project folder; `session/load` resume with self-healing fallback; permission and elicitation mapped to cards; `configOptions` applied live, queued mid-turn; a shared stdin handle sends `session/cancel` when the turn owns the slot. |
| `crates/ui` | GPUI views and presentation state: shell, sidebar (title/project/time rows), transcript (Rewind on a finished turn), rewind card, markdown, composer (project chip and queue), input, cards. |
| `crates/app` | Composition root + supervisor: spawns the daemon, connects, reopens the last session, opens the window; quit closes stdin and waits before killing. |

Dependency direction is one way. The UI never imports a provider or the core.

## 6. Transcript rendering rules (learned by breaking them)

Written up in `docs/ui.md`; short version:

1. The scroll container is a GPUI `List` of turn rows, not a flex column.
2. Rows are `w_full` + `min_w_0`; segments are `flex_shrink_0`.
3. Provider rows use `min_h`, never a fixed `h`: they hug and push.
4. GPUI splits text on `\n` whatever `white_space` says, so a collapsed row's
   detail is flattened (`transcript::flatten`). This was the real cause of the
   overlap that survived PR #27.
5. The list resets while a turn runs; bottom alignment is what makes that read as
   "follow the stream".

Waku is **GPL-3.0**, Circulo is **MIT**: study the approach, never copy code.

## 7. Decisions log

- **Daemon from day 1** (PRD §7): mobile and remote are clients of the same core.
- **WebSocket, not NDJSON**: same contract, `wss://` later behind TLS.
- **OpenCode over ACP, not the v2 HTTP API**: `opencode serve` returns 401 without
  a service token and its pairing flow is undocumented for third parties.
- **Permissions and questions block the turn** and are answered over the same
  connection; the broker hands every request an id (`p-`/`q-`), and a closed
  channel means cancelled.
- **The agent owns remembered decisions**: Circulo sends the chosen `optionId`
  and stores nothing.
- **The provider's session key is opaque to the core**: the driver returns it
  per turn, the core stores it in `sessions.provider_session` and hands it
  back; nothing reads its content. A lost session degrades to a fresh
  conversation — continuity self-heals, it never fails a turn.
- **Checkpoints are private refs, not commits on the user's branch**: a turn
  snapshot is a dangling commit under
  `refs/circulo/session-<uuid>/turn-<n>-start|end`, captured with a temporary
  index so the user's staged and unstaged state is untouched. Restore is exact
  (`git restore`, `git clean -fd`, index reset) and destructive; the UI must
  confirm.
- **Capture is start and end, not end only**: an edit between turns belongs to
  neither response, so the start snapshot is what a rewind to "before this turn"
  restores.
- **ACP has no provider rewind**: after a file rewind the honest v1 drops the
  stored provider session key; the next prompt starts a fresh agent conversation
  over the restored files. The confirmation says so.
- **Worktrees are optional**: checkpoints work in the project folder itself;
  worktrees per session come later without changing the ref format.
- **Rewind moves files, transcript and agent context together, honestly**: the
  worktree restores exactly, `trim_turns` removes the later turns/parts in one
  transaction, the title clears when nothing remains, the provider key is
  dropped and `Driver::reset` kills the child, so the next prompt starts a fresh
  agent conversation over the restored files. ACP has no rewind/fork; the card
  says so.
- **A rewind owns the session while it runs**: it refuses with a turn in flight
  or another rewind, clears the queue and stops the worker, and is the only
  destructive action behind a confirmation card with Cancel as the default.
- **Abort is an outcome, not a failure**: `DriverError::cancelled` becomes
  `FinishReason::Aborted`, with no error banner, and cards the turn left open
  go away. The broker entry owns its session and turn, so cancelling one session
  never closes another's card. ACP has no steer; queue/steer is below.
- **Queue lives in the core, one FIFO per session**: a worker drains it and
  emits `TurnStarted` only when a prompt really starts. `TurnStarted` carries
  the prompt (wire v3), so a queued turn shows what it asked without the client
  guessing. The daemon pushes the whole queue after every change; the window
  draws it and can remove a row. Queued prompts are memory-only.
- **Steer is a redirect, not an injection**: ACP cannot add a prompt to a
  running turn, so `Steer` puts the text at the front and sends
  `session/cancel`; the provider ends the turn and the text is the next
  `session/prompt` with the same context. A provider that cannot abort falls
  back to running it after the current turn. A closing connection clears its
  queues and aborts its turn.
- **A project is a canonical folder, chosen in the composer**: the sidebar row
  stays title / project / time, as in Waku, and the project chip in the composer
  is the picker. Choosing a folder starts or reuses the blank session for that
  folder; a started session is never repointed. `New Task` stays in the current
  project and reuses its blank task. The project list is not a sidebar section.
- **`ProjectAdded` carries the canonical path**: the picker's path and the
  stored path can differ (`/tmp` vs `/private/tmp`), so the daemon answers with
  the path it actually stored before the refreshed list, and the client starts
  the session there.
- **`SessionRef.cwd` is the session's project**: one OpenCode child per session
  in its folder; a projectless session falls back to the daemon's folder.
  `options`/`set_option` keep the fallback because they only discover/apply
  agent-global settings, and the stored overrides land on the next real turn.
- **Mid-turn switches queue to the turn's end**: the picker updates at once
  (promised value), the agent reads it from the next turn. A live, idle
  session applies directly.
- **Replays are skipped, not rendered**: `session/load` history already
  lives in the database; drawing the agent's replay again would double it.
- **The driver cleans its own names**: `ConfigChoice.provider` and the
  stripped display label come from the driver, which knows its protocol;
  the UI groups and paints, and never parses ids.
- **Settings are opaque JSON to the core**: the shell decides what a key
  means; the store and the wire carry the map whole.
- **SQLite with rusqlite (bundled)** at `~/.circulo/circulo.db`; `CIRCULO_HOME`
  overrides. A daemon without a database still serves turns.
- **`serde_json` `preserve_order` in the provider**, so a form's fields keep the
  agent's order instead of alphabetical.
- **Remote**: tailcat recommended (no account, `TailcatKit` for iOS), tailnet and
  `wss://` supported, token stays mandatory.
- **MIT license**, no Waku code, no GPL contamination.

## 8. Known quirks

- A local process keeps probing the daemon port with non-WebSocket requests; the
  daemon logs one line per probe. Harmless.
- The composer's input has no selection, undo, IME or paste; the caret is a block
  painted as a `TextRun` background.
- A turn can be stopped, queued or steered; a queued prompt is memory-only and
  disappears with the daemon, while the turns that did run are in SQLite.
- A steer aborts the current turn and starts the new text as the next prompt;
  it is a redirect, not an injection into the running turn.
- History stores prompts and parts, not the finish reason: reopening a session
  shows every stored turn as `Worked`, including one that was stopped.
- The branch in the composer footer is real when the project is a repository;
  a plain folder hides it. There is no branch picker yet. The project chip next
  to it is real and is the picker.
- Checkpoint refs accumulate per turn; there is no session delete and no ref
  cleanup yet. A checkpoint is a whole-worktree `git add -A` twice per turn, so a
  huge repo pays for it.
- Rewind is file-exact but has no undo: the transcript is trimmed, and the older
  checkpoint refs remain but there is no history UI. The agent's provider session
  is reset, because ACP has no rewind/fork; the card says so.
- A rewind of turn 1 leaves the session blank and clears its title, but blank
  rows are not deleted.
- Blank sessions are reused per project (as in Waku), but there is no delete
  session yet: a blank task for a project the user abandoned stays in the list.
- `RemoveProject` exists on the wire; the composer menu does not expose removal
  yet. Removing a project never touches its folder or its sessions.
- Icons are Kubai Icons (MIT): two-tone where the library has one, line for
  arrows/chevrons/reasoning/panels. They are static SVGs; GPUI does not run the
  library's hover motion.
- Links render in the accent colour and are not clickable (no hit-testing in
  GPUI 0.2.2).
- Light theme values are approximations; this machine is dark.
- `docs/ui.md` owns the tokens and geometry; keep them there.
- The mock labels left in `mock.rs` are the model, access mode, agent mode and
  the composer's picker text.

## 9. Docs map

`docs/`: `prd.md`, `trd.md`, `ux.md`, `ui.md` (tokens, components, transcript
rules), `flow.md`, `implement.md` (phases; Phase 1 is done except attachments),
`permissions.md` (cards: reference design, ACP contract, wire, failure modes),
`projects.md` (canonical folders, composer picker, per-session cwd),
`git.md` (checkpoint refs, capture, restore, rewind plan),
`providers/opencode.md` (ACP, continuity, project folder, cancellation),
`remote.md`.

## 10. Repo flow (do not skip)

- `main` is protected: branch → PR → green CI → **merge commit** → delete branch.
  Never push to `main`.
- Commit messages `type(scope): summary`; one logical change per commit.
- Docs travel with the change.
- The current worktree is `fix/icon-line` (Kubai line icons), not yet merged;
  `origin/main` is the truth, and the next branch starts from it.
