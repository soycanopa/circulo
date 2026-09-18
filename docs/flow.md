# circuloGo — Flows

- **Status:** Draft v0.1 (2026-09-02)
- **Related:** [TRD](trd.md) (endpoints, protocol) · [UX](ux.md)

All flows reference the neutral protocol events from TRD §5 and the relay endpoints
from TRD §4. `UI` = React frontend, `RELAY` = internal/relay, `ORCH` =
internal/orchestrator, `OC` = OpenCode server (opencode serve).

## 1. App start

```
UI (loads)                    main.go / ORCH                     store
   │  GET /agent/projects ─────▶│                                 │
   │                            │── read settings.json ──────────▶│
   │◀── projects[] + status ────│                                 │
   │  EventSource /agent/sse ───▶│ (SSE open, buffered events)    │
   │◀── adapter.status{running} for each auto-start project      │
```

- On startup the orchestrator starts adapters for every stored project in `managed`
  mode (sequentially, not parallel — port + CPU friendly), `attach` projects only
  health-check.
- The UI opens the SSE bridge *before* requesting data so no event is lost between
  list and subscribe (relay buffers events per client until subscribers attach).

## 2. Add project (managed mode)

```
UI                     RELAY            ORCH                opencode serve
 │ POST /agent/projects  │                │                      │
 │ {path, mode}          │── AddProject ─▶│                      │
 │                       │                │── free port (net)    │
 │                       │                │── spawn cmd.Dir=path ▶
 │◀─ 202 {project, adapter.status: starting} ─┐                │
 │                       │                │◀─ GET /global/health 200 (poll ≤10s)
 │◀─ SSE adapter.status{running} ────────────┘                │
 │ GET /agent/projects/:id/meta → agents + providers+models   │
```

Failure paths: binary missing (`exec.LookPath`), port bind fail (retry once), health
timeout → `adapter.status{error, detail}` + stderr tail in `detail`.

## 3. Send prompt (the core loop)

```
UI                    RELAY                 ORCH/adapter            OC
 │ POST …/prompt        │                      │                    │
 │ {text, agent, model} │── Prompt ───────────▶│                    │
 │                      │                      │── POST /session    │ (if new)
 │                      │                      │── POST …/prompt_async (204)
 │ optimistically appends the user message locally (id = client uuid)
 │                      │                      │◀══ SSE ════════════│
 │◀═ SSE part.updated (user text echo replaces optimistic msg by id)
 │◀═ SSE session.status{busy}
 │◀═ SSE part.updated step-start / text(empty) / part.delta × N
 │◀═ SSE part.updated tool(pending→running→completed)
 │◀═ SSE part.updated reasoning…
 │◀═ SSE part.updated text(final full) → step-finish
 │◀═ SSE message.updated (tokens, cost, finish)
 │◀═ SSE session.status{idle} + session.idle-folded
```

Invariants:
- The optimistic user message is keyed by client uuid; the server echo
  (`part.updated` for the user text part carrying its real `messageID`) replaces it.
  Until the echo arrives, the optimistic bubble renders with `pending` opacity.
- Abort: `POST …/abort` → OC emits final message + `session.status idle`;
  UI sets turn state `interrupted` (does not delete partial output).

## 4. Permission flow

```
OC ── permission.asked {id, type, title, metadata} ──▶ adapter ──▶ SSE permission.request ──▶ UI
UI: card above composer [Allow once | Always | Deny]
UI ── POST …/permissions/:pid {response:"once"|"always"|"reject"} ─▶ RELAY ─▶ adapter ─▶ OC
OC ── permission.replied ─▶ adapter ─▶ SSE permission.resolved ─▶ UI removes card
```

- Pending permission survives session switch: cards live in `SessionState`, so coming
  back to the session still shows them (they also live on the OC server).
- If a turn finishes with an unanswered permission, the card persists and the session
  status stays `busy`-ish per server; UI renders "waiting for permission" chip.

## 5. Open session / hydration

```
UI click session s ──▶ GET …/sessions/s/messages ─▶ RELAY ─▶ adapter ─▶ GET /session/s/message
UI receives {info, parts}[] ── reducer builds messages+partOrder (same path as live)
UI ──▶ (live SSE already flowing) any part.updated for s upserts in place
```

Race rule: hydration response and live events may interleave; the reducer upserts by
`(messageID, partID)` and hydrate only *creates missing* messages — a live `part.updated`
for a not-yet-hydrated message creates a stub message that hydrate fills. Order settles
by first-seen + server timestamps.

## 6. Session list maintenance

- `GET …/sessions` on project open; then maintained purely by SSE
  `session.updated` / `session.removed` (created on first prompt by OC as well —
  adapter forwards `session.created`).
- Grouping by `time.updated` (Today/Yesterday/This week/Older) is a UI derived view.

## 7. Reconnection (webview ↔ relay)

```
EventSource error ──▶ retry with backoff (250ms→1s→2s→5s cap, jitter)
on reopen: UI re-FETCHes active session messages + sessions list (state resync),
           reducer merges by id (idempotent — full-part semantics)
```

Adapter ↔ OC reconnect is the same pattern inside the adapter (with
`adapter.status{starting}` surfaced while reconnecting); missed OC events self-heal
because every `message.part.updated` is a full part (TRD §3.3).

## 8. Quit

```
main.go app.Run returns / ServiceShutdown
 └─ ORCH.Shutdown: for each managed adapter: SIGTERM → wait 2s → SIGKILL; persist store
```

## 9. Error taxonomy (user-visible)

| Source | Example | Surface |
|---|---|---|
| Spawn | opencode not on PATH | project banner + `[Retry]` (FR-19) |
| HTTP 4xx/5xx from OC | session gone | toast/status chip + inline retry where actionable |
| `session.error` | provider 429 after retries | inline error block in transcript (FR-21) |
| SSE bridge drop | dev reload | top bar "Reconnecting…" (auto) |
| Unknown event/part | future OC version | ignored (debug log) — no UI noise (NFR-3) |
