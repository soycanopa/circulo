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
	TerminalState,
	ToolCall,
} from "@/types/acp"

export const projectPathAtom = atom<string | null>(null)
export const connectionGenerationAtom = atom<number | null>(null)
/**
 * Single source of truth for the session bound to the composer. Set only by the
 * ACP bridge reducer. UI never writes to this atom directly.
 */
export const activeSessionIdAtom = atom<string | null>(null)
export const agentConnectedAtom = atom(false)
export const sessionStatusAtom = atom<SessionStatus>("idle")
export const configOptionsAtom = atom<ConfigOption[]>([])

/** Per-session UI state, keyed by session_id. Mirrors the Rust `SessionHandle` map. */
export interface ContextUsage {
	used: number
	size: number
}

export interface SessionUiState {
	messages: ChatMessage[]
	streaming: string
	promptInFlight: boolean
	status: SessionStatus
	configOptions: ConfigOption[]
	contextUsage: ContextUsage | null
}
export const sessionsAtom = atom<Record<string, SessionUiState>>({})
/** Messages loaded from disk for read-only history view. */
export const historyMessagesAtom = atom<ChatMessage[]>([])
/** Selectors for the active chat so existing UI keeps working unchanged. */
export const visibleMessagesAtom = atom((get) => {
	const historySid = get(historyViewSessionIdAtom)
	const sid = get(activeSessionIdAtom)
	if (historySid && (!sid || historySid !== sid)) {
		return get(historyMessagesAtom)
	}
	if (sid) return get(sessionsAtom)[sid]?.messages ?? []
	if (historySid) return get(historyMessagesAtom)
	return []
})
export const visibleStreamingAtom = atom((get) => {
	const sid = get(activeSessionIdAtom)
	if (!sid) return ""
	return get(sessionsAtom)[sid]?.streaming ?? ""
})
export const visiblePromptInFlightAtom = atom((get) => {
	const sid = get(activeSessionIdAtom)
	if (!sid) return false
	return get(sessionsAtom)[sid]?.promptInFlight ?? false
})
export const visibleConfigOptionsAtom = atom((get) => {
	const sid = get(activeSessionIdAtom)
	if (!sid) return get(configOptionsAtom)
	return get(sessionsAtom)[sid]?.configOptions ?? get(configOptionsAtom)
})
export const visibleSessionStatusAtom = atom((get) => {
	const sid = get(activeSessionIdAtom)
	if (!sid) return get(sessionStatusAtom)
	return get(sessionsAtom)[sid]?.status ?? get(sessionStatusAtom)
})

export const visibleContextUsageAtom = atom((get) => {
	const sid = get(activeSessionIdAtom)
	if (!sid) return null
	return get(sessionsAtom)[sid]?.contextUsage ?? null
})

export const capabilitiesAtom = atom<AgentCapabilities | null>(null)
export const activePermissionAtom = atom<PermissionRequest | null>(null)
/** FIFO queue of permission requests for the current session — first one is the active card. */
export const pendingPermissionsAtom = atom<PermissionRequest[]>([])
export const errorMessageAtom = atom<string | null>(null)
export const progressMessageAtom = atom<string | null>(null)

export interface WarmTimings {
	initializeMs?: number
	prewarmMs?: number
	configRefreshMs?: number
}

export const warmTimingsAtom = atom<WarmTimings>({})
export const opencodeStatusAtom = atom<OpencodeStatus | null>(null)
/** Resolved `~/.circulo/chats` path for the general Chats section. */
export const generalChatsPathAtom = atom<string | null>(null)
/** Sessions that belong only to the general chats workspace. */
export const generalChatSessionsAtom = atom<ChatSessionSummary[]>([])
/** Sessions nested under each project path (never mixed into general Chats). */
export const projectChatsByPathAtom = atom<Record<string, ChatSessionSummary[]>>(
	{},
)
/** Saved chat opened for read-only history (no live ACP session). */
export const historyViewSessionIdAtom = atom<string | null>(null)
export const appSettingsAtom = atom<AppSettings | null>(null)
export const diffPanelOpenAtom = atom(false)
export interface UserTerminalTab {
	id: string
	title: string
}

export const terminalDrawerOpenAtom = atom(false)
export const userTerminalTabsAtom = atom<UserTerminalTab[]>([])
export const activeTerminalIdAtom = atom<string | null>(null)
export const terminalsAtom = atom<Record<string, TerminalState>>({})
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

export const TERMINAL_DRAWER_HEIGHT_DEFAULT = 256

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

/**
 * Clear live session UI when switching workspace (not when re-opening the same path).
 * Does **not** clear general/project chat indexes — those are multi-workspace lists.
 */
export const resetWorkspaceUiAtom = atom(null, (_get, set) => {
	set(connectionGenerationAtom, null)
	set(historyViewSessionIdAtom, null)
	set(historyMessagesAtom, [])
	set(activeSessionIdAtom, null)
	set(sessionsAtom, {})
	set(activePermissionAtom, null)
	set(pendingPermissionsAtom, [])
	set(configOptionsAtom, [])
	set(capabilitiesAtom, null)
	set(sessionStatusAtom, "idle")
	set(agentConnectedAtom, false)
	set(progressMessageAtom, "Opening workspace…")
	set(warmTimingsAtom, {})
	set(terminalsAtom, {})
	set(userTerminalTabsAtom, [])
	set(terminalDrawerOpenAtom, false)
	set(activeTerminalIdAtom, null)
})
