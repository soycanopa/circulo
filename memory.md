# memory.md — where Circulo is and what comes next

Working notes for the next conversation. Read this first, then `AGENTS.md` and `docs/`.

Last updated: 2026-11-21, after the markdown, tool-id and pin-to-bottom work merged.

## 1. What works today

- **Desktop app** (Rust + GPUI): replica of the Waku reference shell — sidebar, empty state, transcript, composer. Tokens and geometry measured from a capture of Waku 0.1.20 on this machine (`docs/ui.md`).
- **Daemon** (`circulo-daemon`): owns the core, prints `{"url":"ws://127.0.0.1:PORT","token":"…"}`, serves the protocol over WebSocket.
- **Client** (`circulo-client`): handshake, requests, streamed messages. The desktop uses it; mobile/remote will reuse it.
- **Real provider**: OpenCode over **Agent Client Protocol** (`opencode acp`, stdio JSON-RPC). Verified against OpenCode 2.3.2. Reasoning, tool calls and text stream into the window.
- **Markdown in the transcript** (`crates/ui/src/markdown.rs`): `pulldown-cmark` → blocks and spans → `StyledText` with one `TextRun` per span. Paragraphs, headings, bullet and ordered lists, quotes, fenced code, rules, bold, italic, inline code, strikethrough, links (coloured, not clickable). Partial input is safe: the fence or `**` marker that has not arrived yet does not break the parse. 15 fixtures in the module, and the demo provider's canned turn exercises the shapes.
- **Tool rows are correlated by call id**: `Part::Tool` carries the provider's `id`, so an update finds the row its call opened even when another tool started in between. Verified with a real OpenCode turn: no spinners left behind, no duplicate rows.
- **The transcript follows the stream**: the list is `ListAlignment::Bottom`, so a streaming turn stays on its newest row; scrolling up sticks and shows a jump-to-latest button above the composer.
- **Lifecycle**: the app spawns the daemon and holds its stdin open; the daemon exits on EOF, so nothing survives the app (verified with SIGKILL).
- 42 tests green; CI runs fmt + clippy + test on macOS.

Run it:

```sh
cargo run -p circulo                                       # demo provider, no auth needed
CIRCULO_PROVIDER=opencode CIRCULO_PROJECT="$PWD" cargo run -p circulo   # real agent
cargo run -p circulo-daemon                                # daemon alone
```

Checks: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`.

Real-provider test (spends a model call): `cargo test -p circulo-provider-opencode --test real -- --ignored`.

## 2. Next task: permission card

**Symptom:** ACP sends `session/request_permission`; the driver answers `cancelled`, so any tool that needs approval never runs.

**Plan:**
1. Protocol: a `ServerMessage` for a permission request (id, tool name, options, detail) and a `ClientRequest` carrying the chosen option. Bump the version only if the shape is not additive.
2. Driver trait: `start_turn` needs a way to ask the UI and wait. The cleanest shape is a callback that blocks on a response channel handed in by the core, so providers never see the protocol.
3. UI: a card above the composer (design in `docs/ux.md` §4), answerable with the keyboard.
4. Test with the OpenCode provider on a tool that requires approval, plus a fixture for the ACP request in the adapter.

## 3. Pending after permissions, in priority order

1. **Composer text input.** The prompt is currently sent automatically at startup (`PROMPT` in `crates/app/src/main.rs`). Needs a real text field, Enter to send, Shift+Enter for newline.
2. **Model / agent picker.** `session/new` returns `configOptions` (model list, agents). Wire `session/set_config_option` and the existing pickers.
3. **Session continuity.** One `opencode acp` process per session instead of per turn, and ACP `session/load` (the agent advertises `loadSession: true`).
4. **Persistence (SQLite)** in `crates/persistence`, owned by the core. Then the sidebar shows real sessions instead of `mock.rs`.
5. **Markdown, second pass.** Tables (a real OpenCode answer wrote one and it renders as pipes), footnotes and clickable links; parse results are rebuilt every frame today, so cache them per segment if transcripts get long.
6. **Remote/pairing.** Decisions in `docs/remote.md`: QR with a saved tailcat key + `--allow`; the desktop owns the tunnel and starts it with the daemon. `tailcat` is **not installed** on this machine. Open question: what the QR payload carries (bare address vs address + token + node key).

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
