# Tasks: fix-composer-models-and-session-delete

## 1. Daemon: local-first session delete

- [x] 1.1 Reorder `delete_session` in `crates/circulo-daemon/src/http.rs`: local row delete (cascade) + `204` before any agent-side work; schedule the adapter delete as detached background work when an agent session id exists
- [x] 1.2 Daemon test: deleting a session with an agent session id returns `204` without invoking the adapter on the request path (fake adapter asserts no synchronous call); never-sent session performs no adapter call at all
- [x] 1.3 Daemon test: background cleanup failure is logged and does not affect the already-returned delete result

## 2. Daemon: catalog pre-warm

- [x] 2.1 After bind in `crates/circulo-daemon/src/main.rs`, spawn a background task that pre-warms `ModelCatalogCache` for enabled providers; log failure without crashing
- [x] 2.2 Daemon test: freshly built state pre-warms enabled providers and skips disabled ones

## 3. App: catalog fetch with retry and real timeout

- [x] 3.1 Add `get_with_timeout` helper and a 10 s timeout path for `list_models` in `crates/circulo-app/src/client.rs`
- [x] 3.2 Extract the phase-2 catalog+preferences fetch of `schedule_refresh` (crates/circulo-app/src/shell.rs) into a reusable async task that applies results (`composer_models`, `enabled_model_ids`, `bootstrap_enabled_models_if_needed`, `apply_session_composer_state`, `sync_composer`)
- [x] 3.3 Trigger that fetch with bounded backoff (1 s → 2 s → 4 s, reset on success) from session activation when `composer_models` is empty
- [x] 3.4 Stop discarding catalog-fetch errors silently: a retry-exhausted failure leaves the composer on the localized empty-catalog label and is recorded in `error` state only when no session is open
- [x] 3.5 App unit tests: empty-catalog state triggers fetch on activation; success populates composer models; repeated activation while empty does not stack unbounded tasks

## 4. App: async create/delete flows

- [x] 4.1 `create_new_session`: replace the blocking `refresh()` with `schedule_refresh(cx)`
- [x] 4.2 `delete_session`: remove the session from `self.sessions` (and clear selection if needed) before spawning the daemon call; follow success with `schedule_refresh(cx)`; on failure show the existing `session.delete_failed` banner and let the refresh restore the list
- [ ] 4.3 Manual pass (docs/FLOWS.md): delete a used session with the agent server down — card disappears immediately, no UI freeze, no error; delete with daemon stopped — error banner, session restored on next refresh

## 5. Copy: No folder

- [x] 5.1 Change `session.without_folder` in `crates/circulo-i18n/locales/en.json` to "No folder"; run existing i18n/app tests touching the label

## 6. Verification

- [x] 6.1 `cargo test -p circulo-daemon -p circulo-app -p circulo-i18n`
- [ ] 6.2 Manual end-to-end: fresh `run-app.sh` boot (cold daemon), create a folder-less session from Home, model picker lists enabled models within a few seconds; sessions with folder still work
