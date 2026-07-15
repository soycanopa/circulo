import { windowDragRegionProps } from "@/hooks/use-window-drag"
import { isTauri, WINDOW_DRAG_STRIP_HEIGHT } from "@/lib/window-chrome"

/** Full-width drag handle across the top chrome row (Tauri deep drag region). */
export function WindowDragStrip() {
	if (!isTauri) return null

	return (
		<div
			data-slot="window-drag-strip"
			{...windowDragRegionProps()}
			className="absolute inset-x-0 top-0 z-[48] cursor-default select-none"
			style={{ height: WINDOW_DRAG_STRIP_HEIGHT }}
			aria-hidden
		/>
	)
}