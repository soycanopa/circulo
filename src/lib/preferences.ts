export const GENERAL_CHAT_PROJECT = "/Users/soycanopa"

const SIDEBAR_WIDTH_KEY = "circulo-sidebar-width"
const RIGHT_PANEL_WIDTH_KEY = "circulo-right-panel-width"
const LAST_MODEL_KEY = "circulo-last-model"

export const SIDEBAR_MIN_WIDTH = 200
export const SIDEBAR_MAX_WIDTH = 500
export const SIDEBAR_DEFAULT_WIDTH = 288

export const RIGHT_PANEL_MIN_WIDTH = 300
export const RIGHT_PANEL_MAX_WIDTH = 720
export const RIGHT_PANEL_DEFAULT_WIDTH = 420

export function getSidebarWidth(): number {
	const stored = Number(localStorage.getItem(SIDEBAR_WIDTH_KEY))
	if (!Number.isFinite(stored)) return SIDEBAR_DEFAULT_WIDTH
	return Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, stored))
}

export function setSidebarWidth(width: number): void {
	localStorage.setItem(SIDEBAR_WIDTH_KEY, String(width))
}

export function getRightPanelWidth(): number {
	const stored = Number(localStorage.getItem(RIGHT_PANEL_WIDTH_KEY))
	if (!Number.isFinite(stored)) return RIGHT_PANEL_DEFAULT_WIDTH
	return Math.min(
		RIGHT_PANEL_MAX_WIDTH,
		Math.max(RIGHT_PANEL_MIN_WIDTH, stored),
	)
}

export function setRightPanelWidth(width: number): void {
	localStorage.setItem(RIGHT_PANEL_WIDTH_KEY, String(width))
}

export function getLastModel(): string | null {
	return localStorage.getItem(LAST_MODEL_KEY)
}

export function setLastModel(value: string): void {
	localStorage.setItem(LAST_MODEL_KEY, value)
}