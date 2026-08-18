## Context

Sessions sidebar exists with Sessions/Groups switcher and persisted `sidebar_view`. This change replaces navigation with Today/Earlier temporal sections per product direction.

## Goals / Non-Goals

**Goals:**

- Partition sessions by local calendar day of activity.
- Update session row: folder or Without Folder + relative duration.
- Remove view switcher and preference stack.
- Update locales and product docs.

**Non-Goals:**

- New project CTA relocation.
- Backend date filters.
- Settings panel.

## Decisions

### 1. Activity timestamp for day boundary

Use `last_message_at` if set, else `created_at`. Same as existing relative-time source.

### 2. Local calendar day via `time::UtcOffset::current_local_offset()`

Compare dates after converting both `now` and activity to local offset. On failure, fall back to UTC offset (+00:00).

### 3. Omit empty sections

If Today has no sessions (after filter), do not render Today header. Same for Earlier.

### 4. Remove `SidebarView` entirely

Delete enum, `PreferencesBody.sidebar_view`, store methods, daemon handlers logic for view. Keep `GET/PUT /v1/preferences` with empty `{}` body for future prefs, or remove endpoints if nothing left — prefer empty struct with serde default.

### 5. Copy: `session.without_folder` → "Without Folder"

Replace `session.no_project` usage in sidebar and composer picker for consistency.

## Risks / Trade-offs

- [No New project in sidebar] → first project creation blocked until composer/Settings slice.
- [Local offset may fail on some systems] → UTC fallback documented and tested.

## Migration Plan

No DB migration. Orphan `preferences.sidebar_view` row ignored.

## Open Questions

Where to add New project CTA (composer vs sidebar).
