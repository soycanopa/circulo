import { atom } from "jotai"
import type {
	AgentCapabilities,
	AppSettings,
	ChatMessage,
	ChatSessionSummary,
	ConfigOption,
	OpencodeStatus,
	PermissionRequest,
	SessionStatus,
	ToolCall,
} from "@/types/acp"

export const projectPathAtom = atom<string | null>(null)
export const sessionIdAtom = atom<string | null>(null)
export const agentConnectedAtom = atom(false)
export const sessionStatusAtom = atom<SessionStatus>("idle")
export const messagesAtom = atom<ChatMessage[]>([])
export const streamingTextAtom = atom("")
export const promptInFlightAtom = atom(false)
export const configOptionsAtom = atom<ConfigOption[]>([])
export const capabilitiesAtom = atom<AgentCapabilities | null>(null)
export const activePermissionAtom = atom<PermissionRequest | null>(null)
export const errorMessageAtom = atom<string | null>(null)
export const progressMessageAtom = atom<string | null>(null)
export const opencodeStatusAtom = atom<OpencodeStatus | null>(null)
export const chatSessionsAtom = atom<ChatSessionSummary[]>([])
/** Saved chat opened for read-only history (no live ACP session). */
export const historyViewSessionIdAtom = atom<string | null>(null)
export const appSettingsAtom = atom<AppSettings | null>(null)
export const diffPanelOpenAtom = atom(false)
export const sidebarOpenAtom = atom(true)
export const selectedDiffToolAtom = atom<ToolCall | null>(null)

const SIDEBAR_WIDTH_KEY = "circulo.sidebarWidth"
const DIFF_PANEL_WIDTH_KEY = "circulo.diffPanelWidth"

export const SIDEBAR_WIDTH_DEFAULT = 288
export const SIDEBAR_WIDTH_MIN = 220
export const SIDEBAR_WIDTH_MAX = 480

export const DIFF_PANEL_WIDTH_DEFAULT = 384
export const DIFF_PANEL_WIDTH_MIN = 280
export const DIFF_PANEL_WIDTH_MAX = 560

function readStoredWidth(key: string, fallback: number): number {
	try {
		const raw = localStorage.getItem(key)
		if (!raw) return fallback
		const value = Number.parseInt(raw, 10)
		return Number.isFinite(value) ? value : fallback
	} catch {
		return fallback
	}
}

function clampWidth(value: number, min: number, max: number): number {
	return Math.min(max, Math.max(min, value))
}

export const sidebarWidthAtom = atom(
	readStoredWidth(SIDEBAR_WIDTH_KEY, SIDEBAR_WIDTH_DEFAULT),
)

export const diffPanelWidthAtom = atom(
	readStoredWidth(DIFF_PANEL_WIDTH_KEY, DIFF_PANEL_WIDTH_DEFAULT),
)

export const setSidebarWidthAtom = atom(null, (_get, set, width: number) => {
	const next = clampWidth(width, SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX)
	localStorage.setItem(SIDEBAR_WIDTH_KEY, String(next))
	set(sidebarWidthAtom, next)
})

export const setDiffPanelWidthAtom = atom(null, (_get, set, width: number) => {
	const next = clampWidth(width, DIFF_PANEL_WIDTH_MIN, DIFF_PANEL_WIDTH_MAX)
	localStorage.setItem(DIFF_PANEL_WIDTH_KEY, String(next))
	set(diffPanelWidthAtom, next)
})

/** Clear chat/session UI when switching workspace (not when re-opening the same path). */
export const resetWorkspaceUiAtom = atom(null, (_get, set) => {
	set(historyViewSessionIdAtom, null)
	set(chatSessionsAtom, [])
	set(sessionIdAtom, null)
	set(messagesAtom, [])
	set(streamingTextAtom, "")
	set(promptInFlightAtom, false)
	set(activePermissionAtom, null)
	set(configOptionsAtom, [])
	set(capabilitiesAtom, null)
	set(sessionStatusAtom, "idle")
	set(agentConnectedAtom, false)
	set(progressMessageAtom, "Opening workspace…")
})
