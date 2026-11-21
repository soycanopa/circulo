# memory.md — where Circulo is and what comes next

Working notes for the next conversation. Read this first, then `AGENTS.md` and `docs/`.

Last updated: 2026-11-21, after SQLite persistence landed; history survives a restart.

## 1. What works today

- **Desktop app** (Rust + GPUI): replica of the Waku reference shell — sidebar, empty state, transcript, composer. Tokens and geometry measured from a capture of Waku 0.1.20 on this machine (`docs/ui.md`).
- **Daemon** (`circulo-daemon`): owns the core, prints `{"url":"ws://127.0.0.1:PORT","token":"…"}`, serves the protocol over WebSocket.
- **Client** (`circulo-client`): handshake, requests, streamed messages. The desktop uses it; mobile/remote will reuse it.
- **Real provider**: OpenCode over **Agent Client Protocol** (`opencode acp`, stdio JSON-RPC). Verified against OpenCode 2.3.2. Reasoning, tool calls and text stream into the window.
- **Markdown in the transcript** (`crates/ui/src/markdown.rs`): `pulldown-cmark` → blocks and spans → `StyledText` with one `TextRun` per span. Paragraphs, headings, bullet and ordered lists, quotes, fenced code, tables, rules, bold, italic, inline code, strikethrough, links (coloured, not clickable). Partial input is safe: the fence or `**` marker that has not arrived yet does not break the parse. 19 fixtures in the module, and the demo provider's canned turn exercises the shapes.
- **Markdown tables** get weighted column widths and the delimiter row's alignment; verified against a real OpenCode answer (Crate / What it owns / right-aligned Files).
- **Tool rows are correlated by call id**: `Part::Tool` carries the provider's `id`, so an update finds the row its call opened even when another tool started in between. Verified with a real OpenCode turn: no spinners left behind, no duplicate rows.
- **The transcript follows the stream**: the list is `ListAlignment::Bottom`, so a streaming turn stays on its newest row; scrolling up sticks and shows a jump-to-latest button above the composer.
- **Lifecycle**: the app spawns the daemon and holds its stdin open; the daemon exits on EOF, so nothing survives the app (verified with SIGKILL).
- 46 tests green; CI runs fmt + clippy + test on macOS.

Run it:

```sh
cargo run -p circulo                                       # demo provider, no auth needed
CIRCULO_PROVIDER=opencode CIRCULO_PROJECT="$PWD" cargo run -p circulo   # real agent
CIRCULO_PROMPT="…" CIRCULO_PROVIDER=opencode … cargo run -p circulo     # a different ask until the composer is wired
cargo run -p circulo-daemon                                # daemon alone
```

Checks: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`.

Real-provider test (spends a model call): `cargo test -p circulo-provider-opencode --test real -- --ignored`.

## 2. Next task: sidebar from the database

**Why:** the transcript now survives a restart, but the sidebar still draws
`mock.rs`. The pieces exist: `ClientRequest::ListSessions` → `SessionSummary`
(id, project, title, updated_at), and the session title comes from the first
prompt.

**Plan:**
1. `live`/`Shell` keeps the session list the daemon sends, refreshed on start
   and after every turn.
2. Sidebar groups by day from `updated_at` (Today / Yesterday / This Month),
   rows show the title, project and relative time; the open session is marked.
3. Clicking a row sends `OpenSession` and hydrates; `New Task` starts a session.
4. The `mock.rs` workspace keeps only what is not in the database yet (project
   name, branch, model labels).

## 3. After that: free-text answers in the question card

**Why:** `TextInput` exists now, so the last hole in `docs/permissions.md` §2 closes:
a question whose schema has a free-text property can be answered instead of
declined.

**Plan:**
1. The question card renders a `TextInput` per `QuestionKind::Text` field, with
   its own caret and focus; one focus at a time, tab or click moves between
   fields.
2. `Submit` sends the typed values in the same `answers` map; drop the "needs a
   typed answer, decline it" branch and `needs_typing`.
3. Verify with the scripted ACP agent: add a second schema property of type
   string (no enum) and assert the value reaches the agent.

## 4. Pending after permissions, in priority order

1. **Model / agent picker.** `session/new` returns `configOptions` (model list, agents). Wire `session/set_config_option` and the existing pickers.
2. **Session continuity.** One `opencode acp` process per session instead of per turn, and ACP `session/load` (the agent advertises `loadSession: true`).
3. **Persistence (SQLite)** in `crates/persistence`, owned by the core. Then the sidebar shows real sessions instead of `mock.rs`.
4. **Clickable links.** GPUI 0.2.2 has no text hit-testing (`index_for_point`) and mouse events carry window coordinates, so a link click needs either a retained text layout plus element bounds, or an inline layout built from word-sized elements. Math rendering is the other gap.
5. **Remote/pairing.** Decisions in `docs/remote.md`: QR with a saved tailcat key + `--allow`; the desktop owns the tunnel and starts it with the daemon. `tailcat` is **not installed** on this machine. Open question: what the QR payload carries (bare address vs address + token + node key).

## 4. Architecture map

| Crate | Role |
|---|---|
| `crates/protocol` | Wire contract: `Hello`, `ClientRequest`, `ServerMessage`, `Part`, version 1. One JSON per WebSocket text frame. Unknown types skipped. |
| `crates/core` | Sessions, turn orchestration behind `Driver`. |
| `crates/daemon` | Loopback WebSocket listener, handshake, thread per connection, provider chosen by `CIRCULO_PROVIDER`. |
| `crates/client` | Handshake + blocking read loop. No GPUI, no core. |
| `crates/providers/driver` | `Driver` trait: `name()`, `start_turn(prompt, emit: &mut dyn FnMut(Part))`. |
| `crates/providers/demo` | Canned turn; default provider. |
| `crates/providers/opencode` | ACP over stdio; per-turn process. |
| `crates/ui` | GPUI views. Presentation only. |
| `crates/app` | Composition root + supervisor; spawns the daemon. |

Dependency direction is one way (see `AGENTS.md`). Do not let the UI import a provider or the core.

## 5. Transcript rendering rules (learned by breaking them)

Written up in `docs/ui.md` §"How the transcript is rendered". Short version:

1. The scroll container is a GPUI `List` of **turn rows**, not a flex column. A scrolling flex column measures children against the viewport and compresses them.
2. Rows are `w_full` + `min_w_0`; segments inside a row are `flex_shrink_0`. A row measured at minimum width reports less height than it paints, and the next row lands on top.
3. Reasoning and tool rows use `min_h`, never a fixed `h`: they hug their content and push what follows.
4. GPUI splits a text element on `\n` **whatever `white_space` says**, so a collapsed row's text is flattened to one line (`transcript::flatten`) before painting. This is what actually caused the overlap that survived PR #27: reasoning with paragraphs painted every line from a 26 px box. The demo turn never showed it because its reasoning had no newlines; only a real OpenCode turn did.
5. A streaming turn changes its row height without changing its row count, so the list is reset while `status.running`.

Waku uses the same shape (`List`, one id per row). Waku is **GPL-3.0** and Circulo is **MIT**: study the approach, never copy a file, function or type.

## 6. Decisions log

- **Daemon from day 1** (PRD §7): mobile and remote are clients of the same core.
- **WebSocket, not NDJSON**: same contract, `wss://` later behind TLS.
- **OpenCode over ACP, not the v2 HTTP API**: `opencode serve` returns 401 without a service token and its pairing flow is undocumented for third parties.
- **Remote**: tailcat as the recommended tunnel (no account, `TailcatKit` for iOS), tailnet/`wss://` supported, token stays mandatory. Pairing: QR + saved key + `--allow`. The desktop owns the tunnel process.
- **MIT license**, no Waku code, no GPL contamination.

## 7. Known quirks

- A local process keeps probing the daemon port with non-WebSocket requests; the daemon logs one line per probe. Harmless.
- The demo provider's `PROMPT` is fixed at startup; the composer is not typeable yet.
- Light theme values are approximations; Waku follows the system appearance and this machine is dark.
- `docs/ui.md` tokens came from a Waku 0.1.20 capture; keep them there, not scattered in code.

## 8. Repo flow (do not skip)

- `main` is protected: branch → PR → green CI → **merge commit** → delete branch. Never push to `main`.
- Commit messages `type(scope): summary`; one logical change per commit.
- Docs travel with the change.
