# Tasks: stale-doc-closes

## 1. TRD — close §6.1 TRD-API-02 mention

- [x] 1.1 Update `docs/TRD.md:165` to remove "decisión abierta" and reference §15.1 resolution

## 2. TRD — rewrite §7.2 adapter open decisions

- [x] 2.1 Replace the five "Decisiones abiertas del adapter" in `docs/TRD.md:253-259` with a "Decisiones resueltas del adapter" summary pointing to the implementing change

## 3. UX-UI — close §11 items 1, 3, 4

- [x] 3.1 Mark "Enter vs Shift+Enter" closed: Enter envía (UX-UI §4.11)
- [x] 3.2 Mark "¿Settings es panel, ventana, o popover?" closed: panel dentro de la ventana principal
- [x] 3.3 Mark "Densidad de SessionItem" closed: nombre + carpeta (o Without Folder) + duración relativa
- [x] 3.4 Keep item 2 (¿El título se edita inline o en un diálogo?) marked as genuinely open, deferred

## 4. FLOWS — close §5 title note

- [x] 4.1 Update `docs/FLOWS.md:77` to reflect `opencode-provider-hardening` §6: auto-title via OpenCode on default titles only

## 5. Verification

- [x] 5.1 `cargo check --workspace` — sanity check, no Rust changes
- [x] 5.2 `cargo test --workspace` — 153 tests still pass
- [x] 5.3 `python3 scripts/check-crate-boundaries.py` — clean
- [x] 5.4 Markdown still parses (spot-check headings structure unchanged)