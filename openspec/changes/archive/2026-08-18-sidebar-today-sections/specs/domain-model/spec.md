## REMOVED Requirements

### Requirement: Sidebar view preference

**Reason**: Sidebar no longer has Sessions/Groups views.
**Migration**: `SidebarView` enum and `preferences.sidebar_view` storage removed; stale DB row harmless.
