## 1. Favorites state management

- [x] 1.1 Add `favoriteModelsAtom` in `apps/desktop/src/renderer/atoms/preferences.ts` using `atomWithStorage` with key `circulo:favoriteModels`, storing `Record<string, string[]>` (projectId → composite `"providerID/modelID"` strings)
- [x] 1.2 Export a `useFavoriteModels` hook (or inline accessor) that returns favorites for the current project

## 2. Icon and indicator changes

- [x] 2.1 Replace the `"reasoning"` text `<span>` with `<BrainIcon>` in both the "Last used" group and provider-grouped model items (lines 323-327 and 356-359 in `prompt-toolbar.tsx`)
- [x] 2.2 Add `aria-label="Reasoning"` to the brain icon container
- [x] 2.3 Replace the right-side `<CheckIcon>` selection indicator with a `<CircleIcon>` dot on the left side of the model name in both item variants (lines 328-330 and 361-363)
- [x] 2.4 Add a transparent placeholder dot for non-selected items to maintain alignment (same size, invisible)
- [x] 2.5 Remove all `CheckIcon` imports and usage from the file if no longer needed

## 3. Star button on model items

- [x] 3.1 Add a star `<button>` on the right side of each model item (where `CheckIcon` was) with `onClick` that calls `stopPropagation()` and toggles the favorite
- [x] 3.2 Use `StarIcon` from lucide-react: outline when not favorited, `fill="currentColor"` when favorited
- [x] 3.3 Wrap the star in a padded button (`p-1`) for adequate hit area

## 4. Favorites section

- [x] 4.1 Accept `favorites: ModelOption[]` and `onToggleFavorite: (value: string) => void` as props in `ModelSelectorList`
- [x] 4.2 Add a "Favorites" `SearchableListPopoverGroup` above "Last used" (only when not searching and favorites.length > 0)
- [x] 4.3 In provider-grouped models, filter out favorited models so they don't appear twice (only when NOT searching)
- [x] 4.4 When searching, skip the Favorites section and include favorited models inline in provider groups with filled star visible

## 5. Wire everything together

- [x] 5.1 In `ModelSelector`, compute `favoriteModels` from the atom for the current project and resolve them to `ModelOption[]` (like `lastUsedModels` is resolved)
- [x] 5.2 Pass `favorites` and `onToggleFavorite` down to `ModelSelectorList`
- [x] 5.3 Implement `onToggleFavorite` in `ModelSelector`: add/remove the composite value from the atom for the current project
- [x] 5.4 Run `bun run check-types` and `bun run lint` from root to verify no errors
- [x] 5.5 Manually verify in the Electron app: open picker, star a model, confirm it moves to Favorites, reload and confirm persistence, verify dot indicator and brain icon
