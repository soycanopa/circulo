# memory.md — where Circulo is and what comes next

Working notes for the next conversation. Read this first, then `AGENTS.md` and `docs/`.

Last updated: 2026-11-21, after PR #27 merged (`e36c3d8`).

## 1. What works today

- **Desktop app** (Rust + GPUI): replica of the Waku reference shell — sidebar, empty state, transcript, composer. Tokens and geometry measured from a capture of Waku 0.1.20 on this machine (`docs/ui.md`).
- **Daemon** (`circulo-daemon`): owns the core, prints `{"url":"ws://127.0.0.1:PORT","token":"…"}`, serves the protocol over WebSocket.
- **Client** (`circulo-client`): handshake, requests, streamed messages. The desktop uses it; mobile/remote will reuse it.
- **Real provider**: OpenCode over **Agent Client Protocol** (`opencode acp`, stdio JSON-RPC). Verified against OpenCode 2.3.2. Reasoning, tool calls and text stream into the window.
- **Lifecycle**: the app spawns the daemon and holds its stdin open; the daemon exits on EOF, so nothing survives the app (verified with SIGKILL).
- 24 tests green; CI runs fmt + clippy + test on macOS.

Run it:

```sh
cargo run -p circulo                                       # demo provider, no auth needed
CIRCULO_PROVIDER=opencode CIRCULO_PROJECT="$PWD" cargo run -p circulo   # real agent
cargo run -p circulo-daemon                                # daemon alone
```

Checks: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`.

Real-provider test (spends a model call): `cargo test -p circulo-provider-opencode --test real -- --ignored`.

## 2. Next task: markdown rendering (the visible gap)

**Symptom:** agent text renders literally — `**bold**`, `` `code` ``, headings and bullets show their markers. Everything else in the transcript looks right.

**Plan (research already done, do not redo it):**

1. Add `pulldown-cmark = "0.13.6"` to `crates/ui`.
2. New `crates/ui/src/markdown.rs`:
   - Parse into blocks: paragraph, heading, bullet, code block, rule. Inline: text, bold, code (italic optional).
   - Render a paragraph with GPUI `StyledText`:
     `StyledText::new(plain).with_default_highlights(&text_style, highlights)` where
     `highlights: impl IntoIterator<Item = (Range<usize>, HighlightStyle)>` over the **plain text byte offsets**.
   - `HighlightStyle` fields are all `Option`: `color`, `font_weight`, `font_style`, `background_color`, `underline`, `strikethrough`, `fade_out`.
3. `TextStyle` has **no `Default`**: build all 15 fields —
   `color: Hsla`, `font_family: SharedString` (".SystemUIFont"), `font_features`, `font_fallbacks: Option<FontFallbacks>`, `font_size: AbsoluteLength`, `line_height: DefiniteLength`, `font_weight`, `font_style`, `background_color`, `underline`, `strikethrough`, `white_space`, `text_overflow`, `text_align`, `line_clamp`.
   Convert with `px(13.).into()` for the lengths.
4. Alternative if `TextStyle` is awkward: `StyledText::with_runs(...)` with `TextRun { len (utf8 bytes), font: Font, color: Hsla, background_color: Option<Hsla>, underline, strikethrough }`.
5. Wire it in `crates/ui/src/transcript.rs` → `segment_view` for `Segment::Text`, replacing the plain `.child(text.clone())`.
6. Verify with the demo provider first (its text has bold, inline code and paragraphs — free, no model call), then once with OpenCode.

Keep markdown parsing tested with fixtures (`**bold**`, `` `code` ``, `# heading`, `- bullet`, fenced code).

## 3. Pending after markdown, in priority order

1. **Permission card.** ACP sends `session/request_permission`; today the driver answers `cancelled`, so tools that need approval never run. Needs a protocol message + UI card above the composer (design in `docs/ux.md`).
2. **Composer text input.** The prompt is currently sent automatically at startup (`PROMPT` in `crates/app/src/main.rs`). Needs a real text field, Enter to send, Shift+Enter for newline.
3. **Model / agent picker.** `session/new` returns `configOptions` (model list, agents). Wire `session/set_config_option` and the existing pickers.
4. **Session continuity.** One `opencode acp` process per session instead of per turn, and ACP `session/load` (the agent advertises `loadSession: true`).
5. **Persistence (SQLite)** in `crates/persistence`, owned by the core. Then the sidebar shows real sessions instead of `mock.rs`.
6. **Pin-to-bottom / scroll UX.** The transcript list resets while a turn runs (needed for correct heights), and is `ListAlignment::Top`; add a scroll-to-bottom affordance like the reference.
7. **Remote/pairing.** Decisions in `docs/remote.md`: QR with a saved tailcat key + `--allow`; the desktop owns the tunnel and starts it with the daemon. `tailcat` is **not installed** on this machine. Open question: what the QR payload carries (bare address vs address + token + node key).

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
3. A streaming turn changes its row height without changing the row count, so the list is reset while `status.running`.

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
