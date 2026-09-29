# OpenCode v2 — Phase 6 Handoff (E2E manual pass + pending captures)

Start-here doc for a fresh session. The migration itself (phases 0–5) is code-complete
on branch `feature/opencode-v2`; what remains is the owner-gated E2E pass and the two
SSE captures that could not be taken from a scratch server.

- **Status:** captures done (2026-09-18, commit d24917f); owner E2E checklist §3 remains
  the gate. §5 decisions still awaiting the owner.
- **Related:** [opencode-v2-migration.md](opencode-v2-migration.md) (verified endpoint/event
  map — read §1.1 first) · [../../AGENTS.md](../AGENTS.md)

## 1. State at handoff

- Branch stack (pushed to `origin` on 2026-09-18, in merge order):
  `fix/post-audit-gaps` → `feature/circulo-general-frame` → `feature/reasoning-and-mode`
  → `feature/opencode-v2` (HEAD). Canonical repo: `github.com/soycanopa/circulo`.
- CI-lite green at HEAD (`go test ./...` + `pnpm vitest` 20/20 + tsc).
- The app currently runs against **opencode v2.0.8** via
  `CIRCULOGO_OPENCODE_BIN=$HOME/.local/opencode-v2/node_modules/@opencode/cli-darwin-arm64/bin/opencode`.
- Environment: global `opencode` CLI is still **v1.18.31** (npm `opencode-ai`) — untouched
  on purpose. v2 lives side-by-side at `~/.local/opencode-v2/…` (npm `@opencode/cli@2.0.8`).
- Provider credentials (v2 server): Z.AI Coding Plan, MiniMax, DeepSeek, xAI ✅ —
  **opencode gateway NOT configured** (models under the `opencode` provider fail with
  `login fail … X-Api-Key`; that error is correct behavior, not a bug).

## 2. Build & run for the pass

```sh
pnpm --dir frontend build
go build -o build/bin/circulo .
CIRCULOGO_OPENCODE_BIN="$HOME/.local/opencode-v2/node_modules/@opencode/cli-darwin-arm64/bin/opencode" \
  ./build/bin/circulo
```

The managed v2 server prints `server password <random>` and listens on a random loopback
port; both live only in the app process (in-memory ring buffer), see §4 for capture options.

## 3. E2E checklist (owner drives; v2 edition of implement.md §E2E)

1. Fresh temp project (`mkdir /tmp/e2e-v2 && cd /tmp/e2e-v2 && git init`) → add in-app.
2. Prompt "list the files, then reply DONE" → tool card runs, text streams, usage footer.
3. Prompt "create notes.txt with 'hi'" → **expect a permission card** (if the server
   auto-approves, capture §4-A first with a config that asks).
4. Abort mid-turn → partial content stays, status idle.
5. Kill the managed server externally → project error state → recovers on retry.
6. Quit → `pgrep -fl "opencode serve"` empty (no orphans).
7. Reopen → sessions hydrate. NOTE: sessions created under v1 may not exist for v2
   (different storage); `GET /api/experimental/migration/v1` can import them — owner
   decision.

Record every deviation; the translator has best-effort handling for permission and
execution-failure events that still needs live confirmation.

## 4. Pending SSE captures — RESOLVED (2026-09-18)

Both shapes were captured from a standalone 2.0.8 server and are now real fixtures with
translator coverage (commit d24917f):

1. **Permission ask/reply** → event name is `permission.asked` (data = the spec's
   `Permission.Request`: `{id, sessionID, action, resources, save, metadata, source}`),
   and after the reply POST the server broadcasts `permission.replied`
   (`{sessionID, requestID, reply}`). Fixture:
   `internal/opencode/testdata/v2-permission-roundtrip.sse`; translator emits the
   existing neutral `permission.request` / `permission.resolved` events, so
   `protocol.ts` needed no changes.
2. **`session.execution.failed`** → data `{sessionID, error:{type, message}}`
   (`status` only for HTTP-backed errors; live example: `provider.no-route` from a bad
   model pin, fires promptly on a fresh session). Fixture:
   `internal/opencode/testdata/v2-execution-failed.sse`.

Phase 0 finding closed: the "auto-approve" repro was config placement — a project
`opencode.json` with the v2 `permissions` ruleset asks as documented
(opencode.ai/v2/docs/permissions: rules `{action, resource, effect}`, action `shell`
replaces v1 `bash`, unmatched tools default to `ask`, `reject` cascades to the session's
pending asks). The legacy `permission:{edit,bash}` object still loads (server normalizes
it to the ruleset — verified via `GET /api/config`).

## 5. Decisions awaiting the owner (do NOT act without approval)

Status as of 2026-09-29 (see the version-audit amendment in
[implement.md](implement.md)):

1. ~~`AGENTS.md` OpenCode-traps section describes v1.~~ **Done** — rewritten for v2
   on branch `docs/agent-rules-sync`, along with the dependency diagram and project
   map that still omitted `internal/omp`.
2. ~~SIGTERM/SIGINT handler so `kill` routes through graceful shutdown.~~ **Done —
   the original claim here was right and the retraction in this file was wrong.**
   Wails does install a default handler whose routing does reach
   `ServiceShutdown()`; what fails is that the graceful teardown does not finish
   before the process exits. Measured twice on 2026-09-29 against opencode 2.0.18:
   after SIGTERM the app was gone and `opencode serve` was still listening on its
   port. `main.go` now installs `shutdownOnSignal`, which stops the adapters
   synchronously and then exits; SIGTERM and SIGINT were both re-measured with no
   orphans afterwards.
3. Import v1 sessions (`/api/experimental/migration/v1`) or start clean.
4. ~~Switch the global CLI to v2.~~ **Moot** — the global `opencode` is already v2
   (2.0.18 as of 2026-09-29); the 2.0.8 side-by-side install is stale.
5. ~~Merge the branch stack to `main`.~~ **Done** — merged, including the omp adapter.

## 6. Definition of done

- E2E checklist §3 passes with no translator deviations. ← pending (owner)
- Both captures taken, fixtures + translator + tests updated, CI-lite green. ← ✅ done
  (d24917f)
- Docs updated (TRD already pinned to 2.0.8; implement.md E2E note; this doc closed).
  ← migration doc + this doc updated; implement.md note pending the checklist result.
