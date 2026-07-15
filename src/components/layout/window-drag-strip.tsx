import { isTauri, WINDOW_DRAG_STRIP_HEIGHT } from "@/lib/window-chrome"

/**
 * Invisible full-width drag handle across the top chrome.
 * Sits behind window controls so traffic lights + buttons stay clickable,
 * while the rest of the title-bar row (sidebar header, app bar, gutters) stays draggable.
 */
export function WindowDragStrip() {
	if (!isTauri) return null

	return (
		<div
			data-slot="window-drag-strip"
			data-tauri-drag-region
			className="pointer-events-auto absolute inset-x-0 top-0 z-40"
			style={{ height: WINDOW_DRAG_STRIP_HEIGHT }}
			aria-hidden
		/>
	)
}