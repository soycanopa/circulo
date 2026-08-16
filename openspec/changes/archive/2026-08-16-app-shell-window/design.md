## Context

See `proposal.md`. GPUI 0.2.2 is on crates.io (`Application`, `TitlebarOptions.appears_transparent`, `traffic_light_position`).

This change is the window shell only. Session data and daemon client wait for `sessions-sidebar`.

## Goals / Non-Goals

**Goals:**

- Compilable `circulo-app` that opens a dark GPUI window.
- `window_options()` encodes transparent title bar + traffic light point.
- Sidebar expanded/rail widths; collapse toggles state.
- `circulo-i18n` loads `en` JSON.

**Non-Goals:**

- Talking to `circulo-daemon`.
- Rendering real sessions or markdown.
- Heroicons asset pipeline.

## Decisions

### 1. Official `gpui` 0.2.2 from crates.io

**Why:** first-party crate, published 2026-08-15. Avoid unofficial forks.

### 2. Transparent title bar + explicit traffic light point

```
TitlebarOptions {
  appears_transparent: true,
  traffic_light_position: point(12px, 14px),
}
```

Hide button sits to the right of that inset (~80px left padding in the top bar).

### 3. Widths

- Expanded sidebar: 260px
- Rail: 80px (TLs + button remain hittable)

### 4. Locale JSON in the i18n crate

`include_str!("../locales/en.json")` parsed once. No Fluent yet.

### 5. Tests without a display

Unit-test window options, widths, and catalog. Do not require a GPU in CI.

## Risks / Trade-offs

- [GPUI compile is heavy / needs Xcode] → required for the product; document Xcode CLT.
- [Hello-world APIs drift] → pin 0.2.2; `window_options` is isolated if fields rename.

## Migration Plan

None.

## Open Questions

Daemon spawn still open.
