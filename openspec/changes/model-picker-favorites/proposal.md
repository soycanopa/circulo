## Why

The model picker currently lacks a way to mark frequently-used models as favorites, forcing users to scroll through provider groups or rely on the ephemeral "Last used" section. The selection indicator (a check on the right) competes for space with the "reasoning" text badge, and the word "reasoning" itself is unnecessarily verbose. A favorites system with clearer visual indicators will make model selection faster and the UI cleaner.

## What Changes

- Add a star icon button on each model item to toggle favorite status
- Favorited models move to a dedicated "Favorites" section at the top of the picker (above "Last used")
- Persist favorite model IDs to local storage per project
- Replace the "reasoning" text badge with a brain icon (`BrainIcon` from lucide-react)
- Replace the right-side `CheckIcon` selection indicator with a dot (`CircleIcon` from lucide-react) on the left side of the model name
- When searching, the Favorites section collapses and favorited models appear inline within their provider groups (with star visible)

## Capabilities

### New Capabilities

- `model-favorites`: Star-based favorites in the model picker with per-project local-storage persistence. Models marked as favorite appear in a "Favorites" section at the top of the list. Clicking the star on an already-favorited model removes it from favorites.
- `model-picker-indicators`: Replace the "reasoning" text badge with a `BrainIcon`, and replace the right-side `CheckIcon` selected-model indicator with a `CircleIcon` dot on the left side of the model name.

### Modified Capabilities

<!-- No existing specs to modify -->

## Impact

- **Affected code**: `apps/desktop/src/renderer/components/chat/prompt-toolbar.tsx` (ModelSelector, ModelSelectorList), `apps/desktop/src/renderer/atoms/preferences.ts` (new favorites atom with storage)
- **New dependencies**: None (lucide-react already used via `@circulo/ui`)
- **Persistence**: New localStorage key `circulo:favoriteModels` (per-project, same pattern as `circulo:projectModels`)
- **No API changes, no server-side changes**
