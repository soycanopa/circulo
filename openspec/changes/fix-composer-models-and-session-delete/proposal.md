# Proposal: fix-composer-models-and-session-delete

## Why

Three defects in the session/composer lifecycle make the app feel broken or slow:

1. **Folder-less sessions never load the model catalog.** The only code path that loads models/preferences (`schedule_refresh` phase 2) fails silently at app boot — either phase 1 bails early, or `GET /v1/models` hits the client's 2 s timeout while the daemon cold-starts the OpenCode server (measured: ~1.8 s cold vs ~1 ms warm). The Home "New Session" flow calls `refresh()`, which never fetches models, so the composer stays on "No models enabled" for the life of the process. Sessions created with a folder go through `schedule_refresh` again and recover — which is exactly the with-folder vs without-folder discrepancy users see.
2. **Deleting a session freezes the UI for seconds.** The daemon's `DELETE /v1/sessions/{id}` runs the OpenCode-side delete (spawning the OpenCode server if needed, 30 s worst-case timeout) *before* deleting the local row, and the app then runs a blocking `refresh()` on the UI thread. The session card only disappears after the whole round trip.
3. **Copy:** the unassigned-folder label reads "Without Folder"; it should read "No folder".

## What Changes

- The model catalog and preferences get their own retryable load path in the app: any activation of a session with an empty catalog triggers a re-fetch; failures are retried with backoff instead of being swallowed.
- `GET /v1/models` client timeout rises from 2 s to 10 s (cold catalog legitimately takes ~2 s+; first call may spawn agent servers).
- The daemon pre-warms the per-provider model catalog cache at startup so the first client request is warm.
- `DELETE /v1/sessions/{id}` becomes local-first: the Circulo row (and messages, cascade) is deleted and `204` returns immediately; the agent-side session delete runs best-effort in the background and its failure is logged, never blocking the response.
- The app removes the deleted session card optimistically (immediately on confirm) and replaces the blocking post-delete `refresh()` with the async `schedule_refresh`; a failed daemon delete surfaces the existing error banner and restores the session on the next refresh.
- `session.without_folder` en locale value changes from "Without Folder" to "No folder".

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `app-shell`: new requirement — the app must load the model catalog and preferences with retry (the composer must never stay empty for the process lifetime because a boot-time fetch failed), and UI-thread refresh paths touched by this change must not block on network IO.
- `local-daemon-api`: "Model catalog endpoint aggregates per-provider entries" gains a startup pre-warm clause; new requirement — session delete is local-first with best-effort, non-blocking agent cleanup.
- `sessions-sidebar`: unassigned-folder label copy changes to "No folder"; new requirement — deleting a session is optimistic in the UI (card disappears immediately, daemon delete is asynchronous, failure is surfaced).

## Impact

- `crates/circulo-app/src/shell.rs` — catalog/preferences load path, `create_new_session`, `delete_session`, replace blocking `refresh()` calls on the touched paths.
- `crates/circulo-app/src/client.rs` — dedicated timeout for `list_models`.
- `crates/circulo-daemon/src/http.rs` — `delete_session` handler reorder; startup pre-warm hook.
- `crates/circulo-daemon/src/main.rs` — kick off catalog pre-warm after bind.
- `crates/circulo-i18n/locales/en.json` — one string value.
- Tests: app-side retry/state unit tests, daemon delete handler test (fake adapter asserting local-first + 204 timing), pre-warm test.

## Non-goals

- No new endpoints; `DELETE /v1/sessions/{id}` keeps its contract (still returns 204 on success).
- No background sync/periodic refresh loop beyond the bounded retry added here.
- Not migrating every `refresh()` call site to async — only the paths this change touches (create/delete session).
- No re-litigating the folder-assignment flow itself.

## Open questions

1. Pre-warm scope: warm both providers' catalogs at daemon startup (adds ~2 s of background daemon work at boot), or only providers enabled in preferences? Proposal: only enabled providers (matches the cache's own skip rule).
2. If the agent-side best-effort delete fails permanently, the agent session is orphaned in OpenCode. Acceptable (matches today's behavior — errors are logged and ignored), but worth an explicit OK.
