# Circulo — Post-MVP roadmap

This document records product priorities after MVP (F0–F10). See [PRD.md](./PRD.md) for explicit non-goals during MVP.

## Decision (v0.1)

**Recommended order for post-MVP work:**

1. **Terminal drawer (ACP `terminal/*`)** — Highest leverage for daily agent use; completes the Palot-like shell without changing agent binaries. OpenCode and other ACP agents can expose terminal sessions; Circulo already has the permission gate and tool UI patterns to extend.

2. **Second ACP agent** — Registry hooks exist in `src-tauri/src/agents/mod.rs`; adding one more binary (e.g. another documented ACP agent) validates multi-agent without building a full switcher UI first.

3. **Automations / scheduler** — Larger product surface; depends on stable session lifecycle and persistence (now in place). Defer until terminal and a second agent prove the orchestration model.

4. **Git commit / PR workflows** — Nice-to-have integration; not blocking core agent chat. Can ship after terminal or in parallel with automations.

5. **Remote agents / HTTP transport** — Explicitly out of MVP positioning (native stdio ACP). Revisit only if users need agents that cannot run locally.

6. **ACP v2 / migration wizards** — Long-term; no action until the ecosystem moves.

## Rationale

| Option | Why this rank |
|--------|----------------|
| Terminal PTY | Closes the biggest UX gap vs Palot; reuses existing shell layout (diff panel pattern); protocol path documented in TRD as optional `terminal/*`. |
| Second agent | Low incremental cost (registry already exists); proves “one protocol, many agents” positioning before heavier UI. |
| Automations | New subsystem (scheduling, triggers, persistence); higher risk and scope. |
| Git workflows | Orthogonal to ACP; users can use the agent for git today. |

## Next concrete milestones

- **v0.2** — Terminal drawer: ACP `terminal/*` bridge + bottom panel in React. *(merged)*
- **v0.3** — Second agent in registry + agent picker in Settings. *(merged)*
- **v0.4** — Automations MVP: saved prompts + command palette runner. *(merged)*

## References

- [PRD.md](./PRD.md) — post-MVP non-goals
- [TRD.md](./TRD.md) — optional ACP methods (`terminal/*`, `session/list`, `fs/*`)
- [UX.md](./UX.md) — terminal drawer, automations, multi-agent switcher
