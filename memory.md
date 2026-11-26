# memory.md — where Circulo is and what comes next

Working notes for the next conversation. Read this first, then `AGENTS.md` and `docs/`.

Last updated: 2026-11-26, on `main` after PR #57. The suite is green (fmt,
clippy and tests). The model-call tests in `tests/real.rs` need provider credit;
Codex's real tests also need the CLI installed. The scripted agents cover the
lifecycle without a model call.

## 0. Handoff in one minute

- **State:** `main` is clean after the display-math slice; fmt, clippy and tests
  are green. No feature branch is open.
- **Next task:** attachments (§4), then remote, then the inline math layout.
- **Backlog:** §4, ordered. The big remaining v1.0 pieces are attachments,
  remote and the inline math layout.
- **Run it:** `cargo run -p circulo` (demo, no auth) or
  `CIRCULO_PROVIDER=opencode CIRCULO_PROJECT="$PWD" cargo run -p circulo`, or
  `CIRCULO_PROVIDER=codex …` when the Codex CLI is installed. Checks and UI
  automation are in §2.
- **Rules:** branch → PR → green CI → merge commit → delete branch. Never push
  to `main`. See §10.
- **First message for the next conversation:** "Lee `memory.md` (desde §0),
  `AGENTS.md` y `docs/`. Estamos en `main` limpio; la próxima tarea es
  attachments (§4) y el backlog está en §4. No pushees a `main`: branch → PR."

## 1. What works today

The app is usable end to end against a real agent: type a prompt, watch the turn,
answer its permission and question cards, queue or steer a follow-up, stop the
turn, quit, and find the session again.

**Window** (Rust + GPUI, `crates/ui`): the reference shell — sidebar, transcript,
composer, right-panel toggle. Tokens and geometry measured from Waku 0.1.20 on
this machine (`docs/ui.md`). Icons are Iconoir (MIT,
`crates/ui/assets/icons/LICENSE`) in the regular line style: 24 grid, stroke
normalized to 1.75 (the stock 1.5 is too thin at 11–16px), `currentColor`, exact
requested size. Static SVGs; GPUI does not run their motion.

**Agent**: OpenCode over Agent Client Protocol (`opencode acp`, stdio JSON-RPC),
verified against **OpenCode 2.3.2**. Reasoning, tool calls and text stream into
the window. The demo provider (`crates/providers/demo`) still works with no auth
and exercises the shapes.

**Second agent — Codex**: `codex app-server` over stdio (JSON-RPC 2.0, one
message per line). One child per Circulo session; the thread id is the stored
key and `thread/resume` continues it after a restart. Approvals
(`item/commandExecution/requestApproval`, `item/fileChange/requestApproval`)
become the same permission cards, MCP form elicitation becomes a question card,
`model/list` feeds the model/effort pickers, `turn/interrupt` aborts, and
`reset` drops the child. Spec in `docs/providers/codex.md`; the real CLI is not
installed on this machine, so its live tests are `--ignored` and the scripted
agent proves the lifecycle.

**Transcript** (`transcript.rs`, `markdown.rs`):
- GPUI `List` of turn rows, bottom-anchored: it follows the stream, and scrolling
  up shows a jump-to-latest button (`docs/ui.md` has the rules).
- Markdown: paragraphs, headings, bullet/ordered/nested lists, quotes, fenced
  code, rules, **tables** (weighted columns, delimiter alignment), **footnotes**
  (superscript refs, numbered list at the end, dangling refs stay literal), bold,
  italic, inline code, strikethrough, links (coloured, underlined, clickable).
  Parse is cached per segment (`markdown::Cached`).
- **Tool rows carry the provider's call id (`Part::Tool.id`), so an update finds
  its row even when another tool started in between.**
- **Tool output travels once, with the finished row**: `Part::Tool.output` is
  `Option<String>`, absent while the call runs and present when it ends, capped
  at 64 KiB. ACP text `content` blocks and Codex `outputDelta`/`aggregatedOutput`
  fill it. A part per chunk would store the same growing string repeatedly, and
  the wire stays additive without a version bump.
- **Usage is live**: the footer shows the context ratio and cost from ACP, or
  cumulative tokens from Codex. The core sends `Usage` after each turn; it is
  not stored, so a reopened session has no gauge until its next turn.
- **Display math is an image, inline math is source**: RaTeX (MIT) lays out
  `$$…$$`, rasterizes transparent PNG at 2× and caches the `RenderImage` by
  source; `$…$` stays a chip because a GPUI text run cannot hold an image. The
  dependency is 0.1.x: kept behind `crates/ui/src/math.rs`, with a 4 KiB source
  cap and a 256-entry cache, so removing it later is one module.
- Each turn shows its own prompt above the answer.

**Cards** (`permission.rs`, `question.rs`, spec in `docs/permissions.md`):
- Permission: ACP `session/request_permission` → card above the composer with the
  agent's own options; allow kinds primary, reject kinds quiet; Enter allows,
  Escape rejects; the turn blocks until answered; a client that leaves cancels.
- Question: ACP `elicitation/create` (form) → choice rows plus free-text fields
  using the composer's input; Submit waits for required fields; Decline and
  Cancel both exist; the first text field takes the keyboard.

**Composer**: typeable (`crates/ui/src/input.rs`), Enter sends (or queues while a
turn runs), Cmd/Ctrl+Enter steers, Shift+Enter breaks the line, block caret,
grows to four lines then scrolls. No selection, undo, IME or paste yet.

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

**Settings**: the `settings` table (SQLite schema 4, added in v3) stores the
client's own key/value JSON — `favorites.model` is a list of choice ids. The wire
carries `GetSettings`, `SetSetting` and a `Settings` message with the whole map;
the shell loads favourites at startup and redraws from the daemon's answer, never
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
Month / month name, title from the first prompt, project folder, and the trailing
time (`Working Ns` while the open turn runs, otherwise relative). A row opens its
history; `New Task` starts a session in the current project and reuses its blank
task.

**Daemon/client lifecycle**: the app spawns the daemon and holds its stdin open;
the daemon exits on EOF. The connection splits into reader + writer threads on
both sides, which is what lets an answer reach the daemon while a turn blocks.

## 2. Run it and verify it

```sh
cargo run -p circulo                                            # demo provider, no auth
CIRCULO_PROVIDER=opencode CIRCULO_PROJECT="$PWD" cargo run -p circulo
CIRCULO_PROVIDER=codex CIRCULO_PROJECT="$PWD" cargo run -p circulo
CIRCULO_HOME=/tmp/circulo-test … cargo run -p circulo           # a throwaway database
CIRCULO_PROMPT="…" … cargo run -p circulo                       # seeds the composer draft
cargo run -p circulo-daemon                                     # daemon alone
```

Checks: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets`,
`cargo test --workspace`. Real-provider tests (spend model calls, need
credit): `cargo test -p circulo-provider-opencode --test real -- --ignored`
and `cargo test -p circulo-provider-codex --test real -- --ignored` (needs the
Codex CLI installed and signed in). No-credit verification: the continuity
suites and the scripted agents.

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

## 3. Last slice: math; next: attachments

**Math landed** with the split the owner approved: display math (`$$…$$`) is
real, inline math (`$…$`) is a mono chip with its source.

Display math uses RaTeX (`ratex-parser` + `ratex-layout` + `ratex-render`,
MIT, 0.1.16, embedded KaTeX fonts). `crates/ui/src/math.rs` parses and lays out
the TeX, rasterizes a transparent PNG at 2×, converts RGBA→BGRA and caches the
`Arc<RenderImage>` by source (max 256 entries, source capped at 4 KiB); the
element is sized in logical points so it stays sharp. The spike rendered
`x^2 + \frac{1}{2} = \sqrt{y}` correctly in 1.4 ms; the real turn in the demo
shows the integral block centered. Invalid or oversized TeX falls back to the
source in a chip. Rendering is synchronous on first sight and cached
afterwards; a very complex equation can still cost on the parsing frame.

Inline math stays a chip because GPUI paints a paragraph as one `StyledText`
and a run cannot hold an image. A real inline layout (text + images + wrapping)
is the next math slice if it earns its keep.

**Next: attachments** (the last Phase 1 persistence piece), then remote.
After that: the inline math layout.

## 4. Backlog (ordered)

Sizes are rough: S = same session, M = a focused PR, L = a feature slice with
its own plan.

### Next
1. **Attachments** — M/L, the last Phase 1 persistence piece.
2. **Inline math layout** — L. Display math is rendered; `$…$` is a source
   chip until a text+image inline layout exists inside a paragraph.

### Control surface / UI polish
5. **Access mode** — M. The composer label is the static mock `Full access`;
   ACP advertises agent modes but no access mode. Decide per provider (a native
   config option where it exists; otherwise keep it honest or hide it).
6. **Composer** — M/L. No paste, undo/redo, selection or IME. The caret is a
   block painted as a `TextRun` background.
7. **Status banners and shortcuts** — M. Starting/error/reconnecting states;
   the keyboard-first pass (session navigation, panels, focus).
8. **Light theme** — S. The values are approximations; capture and verify on a
   light machine.

### Sessions / projects
9. **Session actions** — M. Delete, rename, pin/archive; the sidebar has no
   context menu. Deletion must also clean `refs/circulo/...` for that session.
10. **RemoveProject UI** — S. The wire request exists; the composer menu does
    not expose it. Removing a project never touches the folder or sessions.
11. **Empty state / no-project onboarding** — M. The centered project picker
    from the reference; `No project` currently lives only in the composer menu.
12. **Sidebar search and project grouping** — M. Waku supports Updated vs
    Project grouping; Circulo only has the date groups. Search is a stub.

### Git
13. **Checkpoint ref cleanup** — S/M once session delete exists; refs accumulate
    per turn today.
14. **Diff / file summaries per checkpoint** — M. The right panel will read the
    refs; `crates/git` does not compute per-turn diffs yet.
15. **Branch picker / worktrees** — L. The footer branch is read-only; worktrees
    per session stay optional and must not break checkpoints or projects.

### Persistence / remote / hardening
16. **Attachments blob store** — M/L, the last Phase 1 persistence piece.
17. **Remote / pairing** — L, post-v1.0 per `docs/remote.md`: tailcat, QR,
    token, desktop owns the tunnel. `tailcat` is not installed here.
18. **Performance + packaging** — L. Long-transcript pass, live CLI smoke tests,
    `.app`/installers, the E2E checklist, and the version line (`Cargo.toml`
    says 0.1.0; tags are v0.3.1 and AGENTS says the Rust line continues at
    v0.4.0 — align at the next release).

### Test gaps
- `tests/real.rs` is ignored until the account has OpenCode credit: a real
  two-turn conversation and a real restart resume. Everything else is covered by
  the scripted agents.
- The real `opencode` 2.3.2 path for steer/rewind has not been run end to end;
  the ACP shapes are covered by the scripted agent.

### Done, do not reopen
- Icons: Iconoir regular line, stroke 1.75, `currentColor`, exact size
  (`docs/ui.md`). The Kubai two-tone/line experiments were rejected.
- Projects, queue/steer, abort, Git checkpoints and rewind are merged and
  documented; the next work builds on them.

## 5. Architecture map

| Crate | Role |
|---|---|
| `crates/protocol` | Wire contract, version 3: `Hello`, `ClientRequest` (+`SetOption`, `CancelTurn`, `Steer`, `RemoveQueued`, `Rewind`, `ListProjects`, `AddProject`, `RemoveProject`), `ServerMessage` (+`Options`, `Settings`, `Queue`, `Projects`, `ProjectAdded`, `RepoStatus`, `Usage`), `TurnStarted` with the prompt, `Part`, `PermissionRequest`, `Question`, `ConfigOption`, `QueuedPrompt`, `ProjectSummary`, `SessionSummary`, `HistoryTurn`. One JSON per WebSocket text frame; unknown types skipped. |
| `crates/core` | Sessions, turn orchestration, one FIFO queue per session drained by a worker, the permission/question broker (entries owned by session + turn), the project list (canonical paths), Git start/end checkpoints around each turn, rewind (restore + trim + driver reset), translation between storage and wire, carrying provider session keys both ways, abort and steer routing. Speaks only the Driver trait. |
| `crates/persistence` | SQLite: sessions (+ provider key, project path), turns, parts, settings, projects; schema v4. `trim_turns` removes a suffix in one transaction. Owned by the core; nobody else opens it. |
| `crates/git` | Whole-worktree snapshots with a temporary index, private `refs/circulo/...` refs, exact restore, and the current branch. Owned by the core. |
| `crates/daemon` | Loopback WebSocket, token handshake, reader + writer per connection, provider chosen by `CIRCULO_PROVIDER` (`demo`, `opencode`, `codex`), database from `CIRCULO_HOME`, one queue worker per session, project/repo status pushes, clear-and-abort on connection close, clean shutdown on stdin EOF. |
| `crates/client` | Handshake, blocking read loop, a splittable writer half, `prompt`/`steer`/`remove_queued`/`cancel_turn`/`rewind` and the project calls. No GPUI. |
| `crates/providers/driver` | `Driver { name, start_turn(SessionRef, prompt, ask, emit) -> Option<key>, cancel(session), reset(session), options(session), set_option(session, id, value), shutdown() }`, `SessionRef { id, resume, cwd }`, and `Asker { permission, question }`. `DriverError` carries whether the turn was cancelled, so the core can report `Aborted` instead of `Failed`. |
| `crates/providers/demo` | Canned turn, no auth. |
| `crates/providers/opencode` | ACP over stdio; one agent process per Circulo session, spawned in the session's project folder; `session/load` resume with self-healing fallback; permission and elicitation mapped to cards; `configOptions` applied live, queued mid-turn; a shared stdin handle sends `session/cancel` when the turn owns the slot. |
| `crates/providers/codex` | `codex app-server` over stdio; one child per Circulo session; `thread/start` / `thread/resume` with the thread id as the key; approvals and MCP form elicitation mapped to the same cards; `model/list` for model + effort; `turn/interrupt` on a shared stdin handle; `reset` drops the child. |
| `crates/ui` | GPUI views and presentation state: shell, sidebar, transcript (link hit-testing, display math, rewind), markdown, composer, cards. |
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

- **Icons are Iconoir regular line, not Kubai**: the Kubai two-tone looked dense
  at 11–16px even after scaling/lifting, and the Kubai line still read as a
  different family. Iconoir's 24-grid line (stroke normalized to 1.75,
  `currentColor`) fits the graphite chrome. Static SVGs; no hover motion.
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
- **Codex uses the app server, not `codex exec --json`**: approvals, interrupt
  and thread resume are first-class there; the batch stream has none of them.
  `approvalPolicy: "on-request"` and `sandbox: "workspace-write"` keep the
  agent in the project and the cards honest. `experimentalApi` is enabled only
  for the documented MCP elicitation request.
- **Links are hit-tested by GPUI, not faked**: `InteractiveText` takes byte
  ranges and reports the clicked index, so the parser keeps `dest_url` and the
  renderer merges adjacent runs into ranges; `cx.open_url` opens the target. No
  webview, no word-sized inline layout.
- **Display math is an image, inline math is source**: RaTeX (MIT) lays out
  `$$…$$`, rasterizes transparent PNG at 2× and caches the `RenderImage` by
  source; `$…$` stays a chip because a GPUI text run cannot hold an image. The
  dependency is 0.1.x, kept behind `crates/ui/src/math.rs` with a 4 KiB source
  cap and a 256-entry cache, so removing it later is one module.
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
- Icons are Iconoir (MIT): regular line, 24 grid, stroke normalized to 1.75
  (stock 1.5 is too thin at 11–16px), `currentColor`, exact size. Static SVGs;
  GPUI does not run their motion. The Kubai two-tone/line experiments are gone.
- A tool row with output expands on click; the output is capped at 64 KiB and
  the cap is visible in the text, not silent.
- Links render in the accent colour with an underline and open in the system
  browser through `InteractiveText` + `cx.open_url`.
- Display math is a cached image; inline math (`$…$`) is a mono chip with its
  TeX source. A very complex equation can still cost on the frame it is first
  parsed; the cache bounds repeats at 256 entries.
- The usage gauge is live only and updates after a turn, not during it; ACP
  reports context and cost, Codex cumulative tokens. A reopened session starts
  without it.
- Light theme values are approximations; this machine is dark.
- `docs/ui.md` owns the tokens and geometry; keep them there.
- `CIRCULO_PROVIDER=codex` runs the Codex adapter; it needs `codex` on `PATH`
  (or `CIRCULO_CODEX_BIN`). `model/list` may need a signed-in account, so the
  pickers can be empty without one. `request_permissions`, URL/device
  elicitation and `tool/requestUserInput` are answered as unsupported; usage is
  not rendered and a `fileChange` diff is not captured (the paths are).
- The Codex CLI is not installed on this machine, so its live tests are
  `--ignored`; the scripted app server in `tests/scripted.rs` proves the
  lifecycle without a model call.
- The mock labels left in `mock.rs` are the model, access mode, agent mode and
  the composer's picker text.

## 9. Docs map

`docs/`: `prd.md`, `trd.md`, `ux.md`, `ui.md` (tokens, components, transcript
rules), `flow.md`, `implement.md` (phases; Phase 1 is done except attachments),
`permissions.md` (cards: reference design, ACP contract, wire, failure modes),
`projects.md` (canonical folders, composer picker, per-session cwd),
`git.md` (checkpoint refs, capture, restore, rewind),
`providers/opencode.md` (ACP, continuity, project folder, cancellation),
`providers/codex.md` (app server, approvals, elicitation, thread resume),
`remote.md`.

## 10. Repo flow (do not skip)

- `main` is protected: branch → PR → green CI → **merge commit** → delete branch.
  Never push to `main`.
- Commit messages `type(scope): summary`; one logical change per commit.
- Docs travel with the change.
- The Codex (#56), tool-output (#57) and usage branches merged; `origin/main`
  is the truth, and the next branch starts from it.
