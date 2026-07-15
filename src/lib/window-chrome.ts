export const APP_BAR_HEIGHT = 46
export const SHELL_INSET = 8
export const CHROME_TOP_OFFSET = 6

const isMac =
	typeof navigator !== "undefined" &&
	(/Mac|iPhone|iPad|iPod/.test(navigator.platform) ||
		navigator.userAgent.includes("Mac OS X"))

export const isTauri =
	typeof window !== "undefined" && "__TAURI_INTERNALS__" in window

/** Native macOS traffic-light origin — synced with tauri.conf.json. */
export const TRAFFIC_LIGHT_POSITION = {
	x: 15,
	y: 15 + CHROME_TOP_OFFSET,
}

/** Sidebar controls row — pairs with traffic lights at y=21. */
export const WINDOW_CONTROL_SIZE = 28
export const WINDOW_CONTROL_TOP = 8 + CHROME_TOP_OFFSET

/** Offset from the window edge where the sidebar toggle starts. */
export const WINDOW_CONTROLS_LEFT = isMac && isTauri ? 93 : SHELL_INSET

/** Total width reserved for traffic lights + window control buttons (window coords). */
export const WINDOW_CONTROLS_INSET = isMac && isTauri ? 160 : 72

/** Full-height chrome strip used for window dragging (inset + app bar). */
export const WINDOW_DRAG_STRIP_HEIGHT = SHELL_INSET + APP_BAR_HEIGHT

export const SIDEBAR_COLLAPSE_THRESHOLD = 600