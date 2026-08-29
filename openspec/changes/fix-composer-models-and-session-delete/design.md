# Design: fix-composer-models-and-session-delete

## Context

See proposal.md — Why. Relevant current state:

- `AppShell::schedule_refresh` (crates/circulo-app/src/shell.rs) is the only loader of `composer_models` + `enabled_model_ids` (phase 2). Phase 1 failure returns early and skips phase 2 forever; phase 2's `list_models()` rides the generic `get()` with a 2 s overall timeout. `unwrap_or_default()` swallows fetch errors.
- `filtered_composer_models()` substitutes placeholder ids (`placeholder/*`) when the catalog is empty; those never match real `enabled_model_ids`, so the composer renders an empty picker.
- `create_new_session` and `delete_session` call the synchronous `refresh()`, which does blocking HTTP on the UI thread.
- Daemon `delete_session` handler (crates/circulo-daemon/src/http.rs) runs the adapter delete (`ensure_running()` may spawn the OpenCode server; client `REQUEST_TIMEOUT` is 30 s) before the local SQLite delete.
- The daemon's `ModelCatalogCache` is lazily populated on the first `/v1/models` request; there is no startup warm-up.

## Goals / Non-Goals

**Goals:**

- A folder-less session gets a working model picker even when the boot-time fetch failed.
- Deleting a session (used or unused) returns `204` quickly and the card disappears immediately.
- The UI thread never blocks on network IO in the create/delete flows touched here.
- Keep the protocol unchanged (no new endpoints, no payload changes).

**Non-Goals:**

- A general background sync/refresh loop.
- Migrating every `refresh()` call site to async (only `create_new_session` and `delete_session` paths are changed; other call sites are left as-is for their own changes).
- Making the agent-side delete guaranteed (it stays best-effort, as today).

## Decisions

### D1: Retry on session activation, not a polling loop

Trigger: `apply_session_composer_state` runs on every session activation; when `composer_models` is empty there, spawn the catalog+preferences fetch (same task shape as `schedule_refresh` phase 2) with a small backoff (e.g. 1 s → 2 s → 4 s, three attempts per activation, resetting on success).

Why not a periodic timer: a poll adds steady daemon load and state complexity for a problem that only exists at boot; activation-triggered retry covers the real user path (open app → start session) with bounded work. Why not only fixing the boot fetch: the daemon may legitimately be cold/down at app boot; activation retry is the safety net that makes the boot race irrelevant.

### D2: Dedicated 10 s timeout for the catalog request

Add `list_models_timeout()` on `DaemonClient` (a `get_with_timeout(path, timeout)` helper) used only by `/v1/models`. 10 s covers cold catalog builds (~2 s measured spawning OpenCode) with headroom for slower machines, without making error states feel hung. Alternatives considered: keep 2 s and rely on daemon pre-warm alone — rejected: pre-warm helps the common case but the app still loses the race whenever the daemon restarts while the app is open; 30 s (adapter's request timeout) — rejected: too long for an honest failure.

### D3: Daemon pre-warms enabled providers at startup

After `TcpListener::bind` succeeds (daemon is answering health), spawn a `tokio::task::spawn_blocking` that runs `model_catalog_cache.get(&registry)` for enabled providers only (the cache already skips disabled ones). Failure is logged; no retry loop at daemon level — the app's D1 retry covers persistent failure. Warming only enabled providers matches the cache's own skip rule and answers open question #1 in the proposal.

### D4: Local-first delete on the daemon

Reorder `delete_session`: read `agent_session_id` + working directory, delete the local row (cascade), respond `204`, then schedule the agent-side delete as `tokio::task::spawn_blocking` with a detached handle. Cap the agent-side client timeout for this call (reuse adapter's 30 s `REQUEST_TIMEOUT` bound; it no longer holds the HTTP request). If the process dies before cleanup, the agent session is orphaned — same end state as today's failure path (logged, ignored).

Alternative rejected: keep agent-first order with a shorter timeout — still blocks the response on a server spawn (~2 s) and fails the local delete when the agent is merely slow.

### D5: Optimistic delete + async refresh in the app

In `delete_session`: remove the card from `self.sessions` and clear selection state immediately (already partially done via `clearing_selection`), then run the daemon delete in the background and follow it with `schedule_refresh(cx)` instead of blocking `refresh()`. On failure, keep the existing error banner; the refresh inside `schedule_refresh` restores the session list (the deleted session reappears), which satisfies the honesty scenario without new state.

`create_new_session` drops its blocking `refresh()` call in favor of `schedule_refresh(cx)` (the async path already covers sessions/projects/messages).

### D6: Locale value change only

`session.without_folder` keeps its key; only the `en` value changes ("Without Folder" → "No folder"). The existing unit test that passes the label as a parameter is unaffected; the e2e-visible string is covered by the sidebar spec delta.

## Risks / Trade-offs

- [Pre-warm adds ~2 s of background CPU at daemon boot] → it runs after bind, off the request path; worst case it delays nothing user-visible.
- [Optimistic delete can show a session as deleted that later reappears on failure] → acceptable and specified: error banner + refresh restore is more honest than a frozen UI; matches spec scenario.
- [Retry on activation could fetch repeatedly while the daemon is down] → bounded: three backoff attempts per activation, no timer; failures surface through the composer's empty-catalog label.
- [Orphaned agent sessions if background cleanup dies] → identical to today's logged-and-ignored failure path; documented in the spec as best-effort.

## Migration Plan

Pure app/daemon behavior fix; no data or protocol migration. Rollback = revert the change; no persisted state depends on it.

## Open Questions

None blocking. (Proposal open question #2 — accepting orphaned agent sessions on best-effort cleanup failure — is answered by "same as today's behavior"; revisit only if OpenCode storage growth becomes a real complaint.)
