# Circulo — UX / UI (MVP)

## Visual direction

Palot-inspired, native Tauri desktop:

| Token | Value |
|-------|--------|
| Sidebar | `#282828` |
| Content | `#161616` |
| Border | white / black at low opacity |
| Text | system UI + mono for code |
| Density | high (desktop agent UI, not marketing site) |

macOS: overlay titlebar + traffic lights (phase 0 shell can be simple; polish later).

## Screens

1. **Empty / onboarding** — Open project CTA; note if OpenCode missing.  
2. **Main shell** — Sidebar (projects) + chat + composer.  
3. **Permission** — Inline in composer area: Allow / Deny.  
4. **Settings (minimal)** — Agent (OpenCode), chats folder, About.

## Components (MVP)

- `AppShell`, `AppSidebar`  
- `ChatView`, `MessageList`, `ChatInput`  
- `PermissionPrompt`  
- `ToolCallCard`  
- `ModelSelector` / config selectors  
- `FileMentionPicker`  

Use shadcn/ui primitives where helpful; keep the tree small.

## Interaction notes

- Composer **disabled** until `session:ready`.  
- New Chat shows a clear loading label (“Creating session…”) without fake session ids.  
- Streaming updates only apply to the active `sessionId`.  
- Errors surface in a non-blocking banner.

## Post-MVP UX

Diff review panel, terminal drawer, command palette, automations, multi-agent switcher.
