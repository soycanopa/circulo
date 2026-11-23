# memory.md — where Circulo is and what comes next

Working notes for the next conversation. Read this first, then `AGENTS.md` and `docs/`.

Last updated: 2026-11-21, model/mode pickers branch. `main` is at `3e94d95`;
83 tests green; CI covers fmt + clippy + test on macOS.

## 1. What works today

The app is usable end to end against a real agent: type a prompt, watch the turn,
answer its permission and question cards, quit, and find the session again.

**Window** (Rust + GPUI, `crates/ui`): the reference shell — sidebar, transcript,
composer, right-panel toggle. Tokens and geometry measured from Waku 0.1.20 on
this machine (`docs/ui.md`).

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
`ConfigOption`/`ConfigChoice`; `SetOption` back). Click opens the agent's
choices with the current one marked, Escape closes, choosing returns the
refreshed list. The driver keeps the choices and re-applies them after every
`session/new`, because a set dies with its agent session (probed: a fresh
session resets `effort` to `default`). Access stays a mock label.

**Storage** (`crates/persistence`, rusqlite bundled): sessions, turns and parts in
`~/.circulo/circulo.db`; `CIRCULO_HOME` overrides the directory. The core writes
on every turn; the app reopens the newest session and hydrates its history.
Storage is **not** agent memory: each turn still starts a fresh process.

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
`cargo test --workspace`. Real-provider test (spends a model call):
`cargo test -p circulo-provider-opencode --test real -- --ignored`.

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
(`FAKE_ACP_MODE=permission|question`). For the app, `CIRCULO_OPENCODE_BIN` points
the daemon at that script.

**OpenCode facts worth remembering**: it auto-runs reads and searches; a
project-level `permission` config does not change ACP behaviour; the ACP session
advertises `loadSession: true` (unused so far).

## 3. Next task: session continuity

**Why:** one `opencode acp` process per turn means the agent never remembers;
history is display-only. The ACP session advertises `loadSession: true`
(unused so far), and the daemon already persists turns in SQLite.

**Plan:**
1. Keep one agent process per Circulo session instead of one per turn: create
   the ACP session once and reuse it across turns.
2. On daemon restart, `session/load` the stored session so the agent resumes
   where it left off.
3. The stored option overrides become the initial `set_config_option` calls on
   the long-lived session; a user switch mid-session applies directly.
4. A switch mid-turn takes effect on the next turn; document it.
5. Verify against OpenCode 2.3.2: two turns in one session share context, a
   restart resumes, and a switch applies without a new process.

## 4. After that, in plan order

1. **Steering and abort** (Phase 2): queue a message mid-turn, cancel a turn.
   Abort must also cancel open permission/question requests.
2. **Git checkpoints and rewind** (Phase 3), with `crates/git` still an empty
   scaffold.
3. **A second provider** (Claude Code or Codex) to prove the Driver trait.
4. **Clickable links and math** in markdown. Links need GPUI text hit-testing,
   which 0.2.2 does not expose: either keep the text layout plus element bounds,
   or build an inline layout from word-sized elements.
5. **Settings (JSON) and the attachment blob store**, the two pieces
   `docs/implement.md` Phase 1 leaves open inside persistence.
6. **Remote/pairing** (`docs/remote.md`): QR with a saved tailcat key + `--allow`;
   the desktop owns the tunnel. `tailcat` is not installed here. Open question:
   what the QR payload carries.

## 5. Architecture map

| Crate | Role |
|---|---|
| `crates/protocol` | Wire contract, version 1: `Hello`, `ClientRequest` (+`SetOption`), `ServerMessage` (+`Options`), `Part`, `PermissionRequest`, `Question`, `ConfigOption`, `SessionSummary`, `HistoryTurn`. One JSON per WebSocket text frame; unknown types skipped. |
| `crates/core` | Sessions, turn orchestration, the permission/question broker, translation between storage and wire. Speaks only the Driver trait. |
| `crates/persistence` | SQLite: sessions, turns, parts. Owned by the core; nobody else opens it. |
| `crates/daemon` | Loopback WebSocket, token handshake, reader + writer per connection, provider chosen by `CIRCULO_PROVIDER`, database from `CIRCULO_HOME`. |
| `crates/client` | Handshake, blocking read loop, a splittable writer half. No GPUI. |
| `crates/providers/driver` | `Driver { name, start_turn(prompt, ask, emit), options(), set_option(id, value) }` and `Asker { permission, question }`. Choices survive per-turn processes: the driver re-applies them. |
| `crates/providers/demo` | Canned turn, no auth. |
| `crates/providers/opencode` | ACP over stdio; one process per turn; permission and elicitation mapped to cards; `configOptions` listed and set, re-applied after every `session/new`. |
| `crates/ui` | GPUI views and presentation state: shell, sidebar, transcript, markdown, composer, input, cards. |
| `crates/app` | Composition root + supervisor: spawns the daemon, connects, reopens the last session, opens the window. |

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
- One agent process per turn: the transcript persists, the agent does not
  remember. Session continuity is the item that fixes it.
- The composer's input has no selection, undo, IME or paste; the caret is a block
  painted as a `TextRun` background.
- Links render in the accent colour and are not clickable (no hit-testing in
  GPUI 0.2.2).
- Light theme values are approximations; this machine is dark.
- `docs/ui.md` owns the tokens and geometry; keep them there.
- The mock labels left in `mock.rs` are the project name, the branch and the
  composer's picker text.

## 9. Docs map

`docs/`: `prd.md`, `trd.md`, `ux.md`, `ui.md` (tokens, components, transcript
rules), `flow.md`, `implement.md` (phases; Phase 1 is done except settings and
attachments), `permissions.md` (cards: reference design, ACP contract, wire,
failure modes), `providers/opencode.md`, `remote.md`.

## 10. Repo flow (do not skip)

- `main` is protected: branch → PR → green CI → **merge commit** → delete branch.
  Never push to `main`.
- Commit messages `type(scope): summary`; one logical change per commit.
- Docs travel with the change.
- The current worktree sits on a merged feature branch; `origin/main` is the
  truth, and the next branch starts from it.
