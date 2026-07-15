import { isTauri } from "@/lib/window-chrome"

/** Tauri drag-region attribute — "deep" lets clicks anywhere in the subtree drag the window. */
export const TAURI_DRAG_REGION_DEEP = "deep" as const

export function windowDragRegionProps(): Record<string, string> | undefined {
	if (!isTauri) return undefined
	return { "data-tauri-drag-region": TAURI_DRAG_REGION_DEEP }
}

/** Opt interactive chrome controls out of the native window drag region. */
export function windowNoDragProps(): Record<string, string> | undefined {
	if (!isTauri) return undefined
	return { "data-tauri-drag-region": "false" }
}