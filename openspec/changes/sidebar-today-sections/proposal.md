## Why

The Sessions/Groups view switcher splits navigation in two modes users must learn. Product direction is a single sidebar with temporal sections: sessions worked on today versus earlier, with clearer folder labeling.

## What Changes

- Remove Sessions/Groups view switcher and `sidebar_view` preference persistence.
- Sidebar lists sessions in two sections: **Today** (activity on the local calendar day) and **Earlier** (activity before today).
- Session cards show title, folder name or localized **Without Folder**, and relative duration on the right.
- Search filters both sections.
- Remove sidebar **New project** CTA (was only in empty Groups view).

## Capabilities

### New Capabilities

- (none)

### Modified Capabilities

- `sessions-sidebar`: Today/Earlier sections, updated session row copy, no view switcher.
- `domain-model`: Remove sidebar view preference from domain.

## Non-goals

- New project creation UI (composer picker or Settings — follow-up).
- Server-side date filtering.
- Changing session/project assignment rules.

## Impact

- `circulo-app`: sidebar UI, time partitioning helpers, locale keys.
- `circulo-core`, `circulo-protocol`, `circulo-persist`, `circulo-daemon`: remove `SidebarView` / preferences field.
- Product docs: PRD, UX-UI, FLOWS, TRD, README, AGENTS.md.

## Open questions (not resolved here)

- Where users create the first project without Groups empty CTA (deferred).
