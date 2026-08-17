# Tasks: live-session-stream

## 1. SSE client

- [x] 1.1 Add `DaemonClient::session_events(session_id)` returning a blocking frame iterator over `GET /v1/sessions/{id}/events` (no overall timeout, 30 s read timeout) plus a parser for `event:`/`data:` frames into `ProtocolEvent`; unit tests feed raw recorded frames (connected, message.created, message.updated, keep-alive, unparsable) through the parser.

## 2. Transcript reducer

- [x] 2.1 Implement the pure `apply_protocol_event(messages, event)` reducer per design D2 (append created, replace updated/completed by id, mark failed, ignore part-level events and connected); unit tests cover incremental updates, unknown ids, failed turns, and out-of-order robustness.

## 3. Stream pump and lifecycle

- [x] 3.1 Wire the reader task (background executor) + std channel + 50 ms timer drain into `AppShell`, applying events via the reducer and notifying; generation counter guards stale events after switching.
- [x] 3.2 Subscription lifecycle: subscribe after `select_session`'s refetch, drop on switch/deselect, resubscribe per selection; manual check that switching mid-turn and returning shows the finished transcript.

## 4. Send flow

- [x] 4.1 Rework `try_send` per design D4: optimistic draft clear, generating derived from live message status, POST kept as background consistency/error anchor with a no-events fallback refetch; unit tests for the generating-state transitions where testable.

## 5. Scroll behavior

- [x] 5.1 Add the `ScrollHandle`, anchored-follow rule (80 px threshold), and the floating "Jump to latest" affordance with the `messages.jump_to_latest` locale key; verify manually that anchored stays bottom and scrolled-up does not jump.

## 6. Resilience

- [x] 6.1 On reader exit: refetch + resubscribe with 1/2/4 s backoff (three attempts), skipping keep-alives and unparsable frames; daemon-down surfaces the existing copy.

## 7. Verification

- [x] 7.0 Fix root keyboard focus discovered during the manual pass: keystrokes never reached the app because no element held GPUI focus (gpui dispatches keys along the focused path only). The shell now owns a root `FocusHandle`, tracks it, and focuses it on open and on composer click; covered by harness tests (`typing_reaches_the_draft_when_composer_focused`).
- [ ] 7.1 Manual pass per FLOWS §6/§6.1 with the real adapter: live typing visible during a long generation, tool cards and task lists appear mid-turn, anchored scroll and jump button, switch-and-return mid-turn, failed turn (kill the OpenCode server) leaves a clean error state, daemon restart recovers.
- [x] 7.2 Full workspace suite + `scripts/check-crate-boundaries.py` green.
