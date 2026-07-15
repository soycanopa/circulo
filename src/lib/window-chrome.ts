export const APP_BAR_HEIGHT = 46
export const SHELL_INSET = 8

const isMac =
	typeof navigator !== "undefined" &&
	(/Mac|iPhone|iPad|iPod/.test(navigator.platform) ||
		navigator.userAgent.includes("Mac OS X"))

export const isTauri =
	typeof window !== "undefined" && "__TAURI_INTERNALS__" in window

export const WINDOW_CONTROL_SIZE = 28
export const WINDOW_CONTROL_GAP = 2

/** Vertical offset from window top to control button top — centered in app bar row. */
export const WINDOW_CONTROL_TOP = SHELL_INSET + (APP_BAR_HEIGHT - WINDOW_CONTROL_SIZE) / 2

/** Native macOS traffic-light origin — synced with tauri.conf.json. */
const TRAFFIC_LIGHT_CLUSTER_HALF = 6
export const TRAFFIC_LIGHT_POSITION = {
	x: 15,
	y: Math.round(WINDOW_CONTROL_TOP + WINDOW_CONTROL_SIZE / 2 - TRAFFIC_LIGHT_CLUSTER_HALF),
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

/** Space between the plus button and session title when the sidebar is collapsed. */
export const APP_BAR_TITLE_GAP = 10

/** Legacy inset — prefer WINDOW_CONTROLS_END + APP_BAR_TITLE_GAP. */
export const WINDOW_CONTROLS_INSET = WINDOW_CONTROLS_END + APP_BAR_TITLE_GAP

/** Full-height chrome strip used for window dragging (inset + app bar). */
export const WINDOW_DRAG_STRIP_HEIGHT = SHELL_INSET + APP_BAR_HEIGHT

export const SIDEBAR_COLLAPSE_THRESHOLD = 600