## Context

The model picker is a popover-based selector rendered inside `PromptToolbar` (`prompt-toolbar.tsx:193-274`). It lists models in two sections: "Last used" (top 3 recent, ephemeral from OpenCode's `model.json`) and provider-grouped models. Selection is indicated by a `CheckIcon` on the right side of each item. Reasoning-capable models display a "reasoning" text badge. There is no favorites mechanism.

State is local React `useState` in both `chat-input.tsx` and `chat-view.tsx`. Recent models come from `useModelState()` via TanStack Query. Per-project model selection persists via `projectModelsAtom` (Jotai `atomWithStorage`, localStorage key `circulo:projectModels`).

The shared popover component is `SearchableListPopover` in `packages/ui/`.

## Goals / Non-Goals

**Goals:**
- Let users mark/unmark models as favorites with a star icon button
- Move favorited models out of provider groups into a dedicated "Favorites" section at the top
- When unfavorited, return the model to its original provider group
- Persist favorite model IDs per project in localStorage
- Replace "reasoning" text badge with `BrainIcon`
- Replace right-side `CheckIcon` with a `CircleIcon` dot on the left side of the model name
- During search, favorites section collapses; favorited models appear inline within provider groups with star visible

**Non-Goals:**
- Drag-and-drop reordering of favorites
- Server-side or cross-device sync of favorites
- Changing how models are fetched, resolved, or the effective-model algorithm
- Modifying the `SearchableListPopover` shared component
- Favorites in the automations dialog picker (only in chat picker)

## Decisions

### 1. Persistence: Jotai `atomWithStorage` per project

Use a new atom `favoriteModelsAtom` with `atomWithStorage` (same pattern as `projectModelsAtom`), keyed `circulo:favoriteModels`. Store as `Record<string, string[]>` mapping project ID → array of composite `"providerID/modelID"` strings.

- **Alternative**: Store in OpenCode's `model.json`. Rejected — that file is OpenCode-managed state, not Circulo's.
- **Alternative**: Jotai atom without persistence. Rejected — favorites should survive app restarts.

### 2. Favorites section behavior

When **not searching**: Favorites section appears at the top (above "Last used"). Favorited models are **removed** from their provider groups — they appear only in Favorites. When unfavorited, they return to their provider group.

When **searching**: Favorites section collapses. Favorited models appear inline within their provider groups with the filled star visible. This avoids showing a nearly-empty "Favorites" group when search filters down to few results.

- **Alternative**: Always show favorites section. Rejected — during search it's jarring to have a section with 1-2 items while provider groups are also filtered.
- **Alternative**: Never show favorites section, only inline stars. Rejected — defeats the purpose of quick access to favorited models.

### 3. Star icon as separate click target

The star button gets its own `onClick` with `stopPropagation()` to prevent triggering model selection. Clicking the model row still selects the model.

- **Icon**: `StarIcon` (outline) for unfavorited, `StarIcon` with `fill="currentColor"` for favorited (lucide-react doesn't have a filled variant natively, use CSS or a separate approach).
- **Placement**: Right side of the row, where `CheckIcon` currently lives.

### 4. Selection indicator: left-side dot

Replace the right-side `CheckIcon` with a `CircleIcon` dot on the **left** side of the model name. Use `size-3` with `fill="currentColor"` in `text-primary`. When not selected, show a transparent/invisible placeholder of same size to maintain alignment.

- **Alternative**: Radio-style circle. Rejected — too heavy for the popover's compact design.
- **Alternative**: No dot, just highlight. Rejected — without a dot, it's unclear which model is selected without reading all names.

### 5. Reasoning indicator: `BrainIcon`

Replace the `"reasoning"` text `<span>` with `BrainIcon` from lucide-react. Keep the `shrink-0 rounded bg-muted p-0.5 text-[10px]` container styling but adapt for icon sizing (`size-3.5`). Add `aria-label="Reasoning"` for accessibility.

- **Alternative**: `BrainCircuitIcon`. Rejected — less recognizable at small sizes.
- **Alternative**: Keep text. Rejected — user explicitly wants icon.

### 6. Item layout order (left to right)

```
[dot indicator] [model name] [brain icon if reasoning] [star button]
```

Provider-grouped items: same layout, minus the star if not favorited.

## Risks / Trade-offs

- **LocalStorage only**: Favorites won't sync across devices. → Acceptable — same limitation as `projectModelsAtom`.
- **Star button hit area**: Small icon in a compact row may be hard to click. → Add padding via a wrapper `<button>` with `p-1` to expand hit area.
- **Favorites removed from provider groups**: Users may look for a favorited model under its provider and not find it. → The Favorites section at top mitigates this by being the first thing users see.
- **No drag-to-reorder**: Favorites order is currently append-based (last favorited last). → Acceptable for v1; can add reordering later.

## Open Questions

- None currently. All design decisions have clear answers based on existing patterns and user requirements.
