export const SHELL_INSET = 8

export const APP_BAR_CONTENT_HEIGHT = 28

/** Fixed app bar chrome row height. */
export const APP_BAR_HEIGHT = 40

/** Top offset for sidebar toggle and plus button (from app bar row start). */
export const APP_BAR_CONTROL_PADDING_TOP = 12

/** Top offset for the floating session title (from app bar row start). */
export const APP_BAR_TITLE_PADDING_TOP = 18

/** Top offset for native traffic lights (from app bar row start). */
export const TRAFFIC_LIGHT_PADDING_TOP = 20

/** Extra inset for the floating session title from the left chrome cluster. */
export const APP_BAR_TITLE_INSET_LEFT = 4

/** Gap between window controls and session title when sidebar is collapsed. */
export const APP_BAR_TITLE_GAP = 12

const isMac =
	typeof navigator !== "undefined" &&
	(/Mac|iPhone|iPad|iPod/.test(navigator.platform) ||
		navigator.userAgent.includes("Mac OS X"))

export const isTauri =
	typeof window !== "undefined" && "__TAURI_INTERNALS__" in window

export const WINDOW_CONTROL_SIZE = 28
export const WINDOW_CONTROL_GAP = 2

/** Vertical offset from window top to control button top. */
export const WINDOW_CONTROL_TOP = SHELL_INSET + APP_BAR_CONTROL_PADDING_TOP

/** Native macOS traffic-light origin — synced with tauri.conf.json. */
export const TRAFFIC_LIGHT_POSITION = {
	x: 18,
	y: SHELL_INSET + TRAFFIC_LIGHT_PADDING_TOP,
}

/** Offset from the window edge where the sidebar toggle starts. */
export const WINDOW_CONTROLS_LEFT = isMac && isTauri ? 93 : SHELL_INSET

/**
 * X where collapsed chrome controls end (toggle + plus), relative to the shell
 * content origin (inside p-2 padding).
 */
export const WINDOW_CONTROLS_END =
	WINDOW_CONTROLS_LEFT -
	SHELL_INSET +
	WINDOW_CONTROL_SIZE +
	WINDOW_CONTROL_GAP +
	WINDOW_CONTROL_SIZE

/** Legacy inset — prefer WINDOW_CONTROLS_END + APP_BAR_TITLE_GAP. */
export const WINDOW_CONTROLS_INSET =
	WINDOW_CONTROLS_END + APP_BAR_TITLE_GAP + APP_BAR_TITLE_INSET_LEFT

/** Full-height chrome strip used for window dragging (inset + app bar). */
export const WINDOW_DRAG_STRIP_HEIGHT = SHELL_INSET + APP_BAR_HEIGHT

export const SIDEBAR_COLLAPSE_THRESHOLD = 600