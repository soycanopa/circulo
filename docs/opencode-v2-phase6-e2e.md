# OpenCode v2 — Phase 6 Handoff (E2E manual pass + pending captures)

Start-here doc for a fresh session. The migration itself (phases 0–5) is code-complete
on branch `feature/opencode-v2`; what remains is the owner-gated E2E pass and the two
SSE captures that could not be taken from a scratch server.

- **Status:** ready to run (2026-09-18)
- **Related:** [opencode-v2-migration.md](opencode-v2-migration.md) (verified endpoint/event
  map — read §1.1 first) · [../../AGENTS.md](../AGENTS.md)

## 1. State at handoff

- Branch stack (all local, none pushed, in merge order):
  `fix/post-audit-gaps` → `feature/circulo-general-frame` → `feature/reasoning-and-mode`
  → `feature/opencode-v2` (HEAD).
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
go build -o build/bin/circulogo .
CIRCULOGO_OPENCODE_BIN="$HOME/.local/opencode-v2/node_modules/@opencode/cli-darwin-arm64/bin/opencode" \
  ./build/bin/circulogo
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

## 4. Pending SSE captures (phase 0 leftovers)

Two event shapes are still unverified against a live server — the translator handles them
best-effort and the fixtures are placeholders:

1. **Permission ask/reply** — expected event name unknown (candidates: `session.permission.…`).
2. **`session.execution.failed`** — data payload unverified (spec: `StructuredError
   {type, message, status}`).

Capture procedure (standalone server, known password — the app's port/password are
in-memory only):

```sh
V2=~/.local/opencode-v2/node_modules/@opencode/cli-darwin-arm64/bin/opencode
mkdir -p /tmp/e2e-capture && cd /tmp/e2e-capture && git init   # git repo may matter for permissions
$V2 serve --hostname 127.0.0.1 --port 5999 2>&1 | tee /tmp/v2-serve.log &
P=$(grep -o 'server password .*' /tmp/v2-serve.log | awk '{print $3}')
curl -s -N -u "opencode:$P" --max-time 90 http://127.0.0.1:5999/api/event > capture.sse
# in another shell: create a session, set a model, send the trigger prompt
```

- **Phase 0 findings to resolve first**: a scratch dir auto-approved edits even with
  `opencode.json` `permission:{edit:"ask",bash:"ask"}` (config placement/merge needs
  investigating — check `GET /api/config` after writing the file), and an invalid-model
  turn hung instead of emitting `execution.failed` (session may need to be fresh).
- Permission event candidate names to watch: anything with `permission` in the `type`.
- After capture: add fixtures under `internal/opencode/testdata/v2-*.sse` (version
  header), extend `translate_v2_test.go`, adjust `TranslateV2` if the real shapes differ,
  and update this doc + the migration doc.

## 5. Decisions awaiting the owner (do NOT act without approval)

1. `AGENTS.md` OpenCode-traps section still describes v1 (full-part updates,
   `permission.asked`, 1.18.25 fixtures) — rewrite for v2 is drafted in conversation,
   not applied.
2. SIGTERM/SIGINT handler so `kill` routes through graceful shutdown (today `pkill`
   orphans the managed server — happened repeatedly during the v2 work).
3. Import v1 sessions (`/api/experimental/migration/v1`) or start clean.
4. Switch the global CLI to v2 (`npm i -g @opencode/cli`) or keep v1 for daily use.
5. Merge the branch stack to `main` (order as listed in §1).

## 6. Definition of done

- E2E checklist §3 passes with no translator deviations.
- Both captures taken, fixtures + translator + tests updated, CI-lite green.
- Docs updated (TRD already pinned to 2.0.8; implement.md E2E note; this doc closed).
