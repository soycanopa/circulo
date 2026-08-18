# Design: live-session-stream

## Context

`AppShell` (crates/circulo-app/src/shell.rs) owns all UI state; `messages: Vec<Message>` holds only the selected session. `try_send` blocks on the daemon POST (30 s) and then refetches — nothing reaches the screen mid-turn. Available GPUI patterns: `cx.spawn(async |this, cx|)`, `background_executor().spawn`, `this.update(cx, …)`, `cx.notify()`. The daemon's SSE (local-daemon-api) sends `server.connected` first, then typed `ProtocolEvent` frames filtered per session, with a 15 s keep-alive; events carry **full message snapshots**, and `select_session` today does a blocking `list_messages` on the UI thread (pre-existing, out of scope here).

The daemon changes nothing in this change. The app is the only surface touched.

## Goals / Non-Goals

**Goals:**

- Replies render progressively in the open session, driven by the daemon's existing SSE.
- Sending feels instant (optimistic draft clear) and the generating state reflects the real turn.
- Scroll behavior follows UX-UI §4 (anchored follow + jump-to-latest).
- Zero new dependencies; no protocol or daemon changes.

**Non-Goals:**

- Background streaming for non-selected sessions (FLOWS §8 full vision — deferred; recorded with the user's tacit acceptance of the recommended scope).
- Cancel, TLS, replay-based reconnect (one refetch covers the gap), composer text-input rework.

## Decisions

### D1. Blocking SSE reader + std channel + timer drain (no new deps)

`DaemonClient::session_events(session_id)` opens `GET /v1/sessions/{id}/events` with `ureq` — no overall timeout, a 30 s read timeout (comfortably above the daemon's 15 s keep-alive) — and returns an iterator of raw frames. A reader task on the background executor loops over frames, parses each `data:` payload into `ProtocolEvent`, and pushes it into a `std::sync::mpsc::channel`. A `cx.spawn` task drains the channel on a 50 ms timer and applies events inside `this.update`. The 50 ms cadence doubles as render batching (deltas coalesce; markdown re-parse happens at most 20×/s).

- Alternative considered: `smol`/`async-channel` for await-based pumping — rejected, adds a dependency for no user-visible gain.

### D2. Upsert-by-id transcript reducer

A pure function `apply_protocol_event(messages: &mut Vec<Message>, event: &ProtocolEvent)`:

- `SessionMessageCreated` → append (if the id is unknown).
- `SessionMessageUpdated` / `SessionMessageCompleted` → replace by id (snapshots are full; parts arrive complete).
- `SessionMessageFailed` → mark the message failed (status error, streaming off).
- `SessionPartAppended` / `SessionPartUpdated` / `SessionToolCallUpdated` → ignore (their data is already inside the message snapshots the daemon emits alongside).
- `ServerConnected` → no-op (handshake confirmation).

Pure reducer = unit-testable without GPUI; the shell only wires it. Unknown-message updates (e.g. a race after switching) are ignored defensively.

### D3. Subscription lifecycle: one stream, generation-guarded

Subscribing happens when a session is selected: initial `list_messages` (existing behavior) then open the stream (D1). Switching sessions increments a generation counter and drops the old stream; the drain loop discards events whose generation is stale, so rapid switching cannot cross-contaminate transcripts. Deselecting closes the stream. Resubscribing always pairs with a refetch because the daemon SSE has no replay.

### D4. Send flow: optimistic, events drive progress

On send: clear the draft, set generating, and fire the POST on the background executor as today — but its result no longer drives the transcript. Generating clears when the reducer sees the assistant message complete/failed for this turn, or when the POST returns an error (error copy surfaced as today). The POST completing successfully with no events seen triggers one refetch (covers a stream that silently died before the turn).

### D5. Anchored auto-scroll + jump-to-latest

The message list gains a `ScrollHandle`. On each applied update, if the handle's offset is within a bottom threshold (~80 px) the list scrolls to bottom; otherwise a floating "Jump to latest" affordance appears over the transcript (new locale key `messages.jump_to_latest`), shown only while the transcript's last message is streaming. Clicking it scrolls to bottom and re-anchors.

### D6. Root keyboard focus (verification-discovered fix)

The manual pass surfaced that typing never reached the app: with no element holding GPUI focus, keystrokes dispatch along the focused path only and die at the dispatch tree root, so the root div's `on_key_down` never fired (reproduced in the gpui test harness, including on a bare window). The shell now owns a root `FocusHandle`: `AppShell::new` focuses it at open, the root div `track_focus`es it, and clicking the composer re-focuses it. Key handling remains the existing flag-based routing.

### D7. Resilience

When the reader task exits (stream EOF, read timeout, or connect failure), it schedules: one transcript refetch, then a resubscribe with backoff 1 s → 2 s → 4 s, max three attempts **per outage**. A `server.connected` handshake resets the attempt counter so later drops get a full budget. If those three attempts fail, the existing sidebar error slot shows `messages.stream_dropped`. If health is unreachable, the existing `sidebar.daemon_down` copy shows and the next app-level refresh resumes the normal ensure-daemon path. Keep-alive/comment frames and unparsable lines are skipped without failing the stream.

The POST's `list_messages` is applied when the stream never spoke **or** any live message is still `is_streaming`, so a turn that streamed partially but missed `completed` still snaps to persisted state. A `schedule_refresh` snapshot is discarded if `stream_gen` advanced while it was in flight, so a late refetch cannot clobber tokens that already arrived on the new subscription.

## Risks / Trade-offs

- [Full-snapshot upsert re-renders markdown on every event] → the 50 ms drain batches events; at MVP transcript sizes this is cheap. If it ever hurts, the reducer can diff before notifying.
- [Blocking `select_session` on the UI thread (pre-existing) gets more visible with live streams] → out of scope here; noted for mvp-hardening.
- [Stale events after fast session switching] → generation counter discards them (D3).
- [Missed events between refetch and subscribe] → the subscribe order is refetch-then-subscribe; the daemon filters by session and the POST-side consistency refetch (D4) closes residual gaps.

## Migration Plan

No storage, protocol, or daemon changes; rollback is restarting the previous app binary. Ship behind nothing — the old path (POST + refetch) remains as the fallback whenever a stream cannot be opened.

## Open Questions

None. Assumptions recorded: selected-session-only scope, 50 ms drain, 3-attempt backoff, 80 px anchor threshold — all tunable in code without spec impact.
