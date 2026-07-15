import { getCurrentWindow } from "@tauri-apps/api/window"
import { isTauri, WINDOW_DRAG_STRIP_HEIGHT } from "@/lib/window-chrome"

/**
 * Full-width drag handle across the top chrome row.
 * Uses startDragging() so window move works without requiring a focus cycle.
 */
export function WindowDragStrip() {
	if (!isTauri) return null

	async function handleMouseDown(event: React.MouseEvent<HTMLDivElement>) {
		if (event.button !== 0) return
		const target = event.target as HTMLElement
		if (target.closest("button, a, input, textarea, select, [data-tauri-drag-region='false']")) {
			return
		}
		try {
			await getCurrentWindow().startDragging()
		} catch {
			// noop outside tauri runtime
		}
	}

	return (
		<div
			data-slot="window-drag-strip"
			data-tauri-drag-region
			onMouseDown={(event) => void handleMouseDown(event)}
			className="absolute inset-x-0 top-0 z-[48] cursor-default select-none"
			style={{ height: WINDOW_DRAG_STRIP_HEIGHT }}
			aria-hidden
		/>
	)
}