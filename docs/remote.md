# Circulo — Remote & Mobile Design (Tailcat)

- **Status:** Design only — **implementation is gated** by the project rule:
  *do not build remote until the local protocol is stable* (gate = full pass of
  the E2E checklist in [implement.md](implement.md)).
- **Chosen transport:** [Tailcat](https://tailscale.com/tailcat) — owner decision
  (2026-09-02). NOT `tailscale serve`/tailnet: Tailcat is the accountless,
  netcat-style CLI built on Tailscale's data plane (WireGuard + NAT traversal +
  DERP relay).
- **Sources of truth:** tailscale.com/tailcat and
  github.com/tailscale/tailcat (README verified 2026-09-02). The project
  explicitly makes **no CLI/wire-format stability promises** and public relays
  may be revoked at any time — pin the tailcat version and re-verify on bump.

## 1. Goal & non-goals

**Goal:** from a phone, watch agent sessions live and — the actual reason this
exists — **approve permission requests** away from the desk.

**Non-goals:** multi-user anything, public-internet exposure, cloud relay of
code/context beyond the peer-to-peer tunnel, background sync services.

## 2. Architecture

```
┌─ desktop (macOS) ──────────────────────────────────────────────┐
│ Circulo (Wails)                                              │
│   orchestrator ── opencode serve ×N   (all on 127.0.0.1)       │
│   relay (/agent: REST + SSE)  ◄── same-origin webview (local)  │
│                                                                │
│   REMOTE LISTENER (single port, gated by user):                │
│     frontend/dist (static UI)  +  relay /agent                 │
│        ▲                                                       │
│   tailcat serve --allow=<nodekey?> <port>                      │
└────────┬───────────────────────────────────────────────────────┘
         │ encrypted P2P (DERP bootstrap → hole-punched direct)
┌────────▼───────────────────────────────────────────────────────┐
│ phone: tailcat client (WASM/Termux/future companion app)       │
│   → browser opens the relayed Circulo UI                     │
└────────────────────────────────────────────────────────────────┘
```

Key properties:

- **One exposure point.** Only the remote listener is tunneled. The
  `opencode serve` processes stay bound to 127.0.0.1 forever — the phone talks
  exclusively to our neutral protocol, never to OpenCode's API.
- **The UI travels with the API.** The remote listener serves `frontend/dist`
  (static React build) next to `/agent`, so the phone gets the full app from
  the same port. No Wails server-mode needed, no second frontend.
- **Agent independence.** The remote surface is the neutral protocol; adapter
  #2 comes free, nothing remote-specific to rebuild.

## 3. Pairing UX (F2)

- Settings → **Remote access** card: `[Start tunnel]` spawns
  `tailcat serve --allow=<nodekey?> <port>` as a managed child process.
- The generated token is displayed **once, as a QR code** (phones pair by
  camera; tokens are short strings). Never written to settings.json, never
  logged (adapter stderr ring buffer must redact `tc…` tokens).
- Default keys are **ephemeral**: the tunnel dies with the app and the token
  becomes dead weight — closing the app IS revocation.
- Optional `genkey --client` on a trusted device + `--allow=nodekey:…` for a
  recurring phone: unlisted handshakes are silently ignored by tailcat.

## 4. Security model (read this twice)

- **Possession of the token = full agent control**, including approving
  permission requests (bash, edits). This is the entire point and the entire
  risk. Mitigations, in order of default-ness:
  1. Ephemeral per-session tokens (default): leaked token is dead in hours.
  2. `--allow` allowlist for known devices; strangers are dropped silently.
  3. Token hygiene: QR display only, redaction in logs, no persistence.
  4. The relay keeps its panic-recovery + 503 gates; nothing new is exposed
     remotely that isn't already served to the local webview.
- The remote listener binds the loopback interface only; **tailcat is the only
  thing that bridges it off-machine**. Never bind 0.0.0.0.
- If we ever need auth *inside* the stream (e.g. shared demo machines), that is
  a relay-level token check — deliberately out of scope while Tailcat's
  address-as-credential covers it.

## 5. Phone side — honest options (2026-09)

Tailcat ships **no iOS/Android binaries today**. Options, worst to best:

| Option | What | Verdict |
|---|---|---|
| WASM client (tailscale.github.io/tailcat) | browser establishes the tunnel, relay-only until WebRTC lands | **Validation path (F1)** — zero install, but experimental and slower |
| Android + Termux | real `tailcat` CLI on the phone | works for tinkerers, not a product |
| **Companion app embedding tailcat** | tailcat is Go + OSS: embed the client library in a small Circulo mobile shell (webview → relayed UI) | **Target (F2/F3)** — the robust path |

Recommendation: validate with WASM, ship the companion app when remote becomes
a daily driver. Re-evaluate if Tailscale ships official mobile tailcat.

## 6. Risks

- **Public relays:** free, rate-limited, no SLA, revocable at any time. For
  reliability: self-host a DERP (`genkey --region=…` / `--derpmap-url`). Direct
  P2P (the common case on a home network) bypasses relays after hole-punching.
- **No stability promises** on CLI flags or wire format → pin the tailcat
  version in docs and in the spawn args; smoke-test the tunnel on version bump
  (same discipline as the OpenCode fixtures).
- **WASM client is experimental** — acceptable for F1 validation, never the
  shipping path.
- **iOS background limits:** SSE does not survive backgrounding. Live streaming
  works with the app foregrounded; anything else needs push (see F3).

## 7. Phases (all gated on "local protocol stable")

| Phase | Content | Gate |
|---|---|---|
| F0 (this doc) | design frozen | — |
| F1 | manual validation: `tailcat serve <port>` in front of a static-UI+relay listener; phone opens the UI, streams a turn, approves a permission | E2E checklist fully green (implement.md) |
| F2 | in-app tunnel management: spawn/stop, QR pairing, allowlist, redaction in logs | F1 validated on real hardware |
| F3 | permission push notifications (ntfy/APNs decision then) — the only genuinely new machinery | F2 in daily use |

What we deliberately do **not** do now: no listener, no tailcat spawn, no UI
card. The codebase change this doc requires is zero.
