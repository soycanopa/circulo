# live-session-stream

## Why

PRD-CHT-02 makes incremental streaming a P0 requirement and the product philosophy calls speed first-class, yet the app still shows a reply only after the whole turn finishes: `try_send` awaits the POST (up to 30 s with the real OpenCode adapter), then refetches. The daemon already exposes the typed per-session SSE stream (`GET /v1/sessions/{id}/events`); the app simply never opens it. This change makes replies appear live on screen.

## What Changes

- The app subscribes to the selected session's event stream and applies protocol events to the transcript as they arrive: message snapshots are upserted by id, so text, tool calls, and task lists appear incrementally while the turn generates.
- Sending becomes optimistic: the draft clears on send, the user message appears via `session.message.created`, and the generating state derives from live message status instead of the POST completing. The POST still runs in the background as a consistency anchor and error surface.
- The transcript follows new content only when the user is anchored at the bottom (UX-UI §4): anchored sessions auto-scroll as content streams; a "Jump to latest" affordance appears when the user has scrolled up during generation.
- Stream resilience: if the event stream drops, the app refetches the transcript once and resubscribes with bounded backoff; if the daemon is unreachable, the existing daemon-down copy is shown. Keep-alive frames are ignored.
- Scope is the **selected session only**: switching sessions resubscribes (with a refetch covering anything missed); turns finishing in background sessions surface on the next refresh. (Records the FLOWS §8 open decision for this change; background multi-session continuation stays future work.)

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `composer-stream`: adds requirements for replies streaming live into the open transcript and for anchored follow behavior while content streams.

## Impact

- Code: `crates/circulo-app` only — `client.rs` (SSE reader), `shell.rs` (event application, subscription lifecycle, send flow, scroll), `locales/en.json` (one new key). No daemon, protocol, persistence, or adapter changes.
- Dependencies: none new; the reader reuses `ureq` and crosses to the UI via a std channel drained on a timer.

## Non-goals

- Background continuation of in-flight turns for non-selected sessions (FLOWS §8 full vision).
- Canceling a generation (stays P1), TLS app↔daemon, `QuestionCard`, reconnect replay beyond one refetch.
- Reworking the composer's manual keyboard capture (separate gap).

## Open questions

None blocking. Recorded assumptions live in `design.md`: selected-session-only scope (user took the recommended option without further input), 50 ms event-drain batching, three-attempt resubscribe backoff.
