import { atom } from "jotai"
import type { SettingsSectionId } from "@/lib/settings-sections"
import type {
	AgentCapabilities,
	AgentDescriptor,
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
	const global = get(configOptionsAtom)
	if (!sid) return global
	const sessionOpts = get(sessionsAtom)[sid]?.configOptions
	// Prefer per-session options; fall back to global when the session slot is
	// still empty (e.g. after agent switch + reconcile race).
	if (sessionOpts && sessionOpts.length > 0) return sessionOpts
	return global
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
/** Cached agent registry from `list_agents` (shared across settings + composer). */
export const agentsAtom = atom<AgentDescriptor[]>([])
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
export const fileTreeOpenAtom = atom(false)
/** Bumped after git operations (branch switch/create) to refresh consumers. */
export const gitRefreshVersionAtom = atom(0)
export interface UserTerminalTab {
	id: string
	title: string
}

export const terminalDrawerOpenAtom = atom(false)
export const userTerminalTabsAtom = atom<UserTerminalTab[]>([])
export const activeTerminalIdAtom = atom<string | null>(null)
export const terminalsAtom = atom<Record<string, TerminalState>>({})
export const sidebarOpenAtom = atom(true)
/** Settings mode: active section rendered in the main area. */
export const settingsSectionAtom = atom<SettingsSectionId>("general")
export const selectedDiffToolAtom = atom<ToolCall | null>(null)

// ---------------------------------------------------------------------------
// Usage tracking (Settings > Usage)
// ---------------------------------------------------------------------------

export interface UsageSample {
	timestamp: number
	/** tokens used in the context window */
	used: number
	/** context window size in tokens */
	size: number
}

/** usage_update samples per session (kept in memory for the current run). */
export const usageHistoryBySessionAtom = atom<Record<string, UsageSample[]>>({})

export interface ToolOutputStats {
	toolCallCount: number
	totalOutputBytes: number
}

/** Aggregate tool-call output volume measured from `tool_call_update`. */
export const toolOutputStatsAtom = atom<ToolOutputStats>({
	toolCallCount: 0,
	totalOutputBytes: 0,
})

export interface McpSavings {
	/** server id -> times loaded via mcp_load/mcp_call */
	loadedServers: Record<string, number>
	/** bytes saved by compact_result (terminal filters, orchestrator) */
	savingsBytes: number
	compactionCount: number
}

/** Measured savings from compaction and MCP usage. */
export const mcpSavingsAtom = atom<McpSavings>({
	loadedServers: {},
	savingsBytes: 0,
	compactionCount: 0,
})

const DRAFTS_KEY = "circulo.drafts"

function readDrafts(): Record<string, string> {
	try {
		const raw = localStorage.getItem(DRAFTS_KEY)
		if (!raw) return {}
		const parsed = JSON.parse(raw) as Record<string, unknown>
		const next: Record<string, string> = {}
		for (const [sessionId, text] of Object.entries(parsed)) {
			if (typeof text === "string") next[sessionId] = text
		}
		return next
	} catch {
		return {}
	}
}

/** Unsent composer text per session id, persisted in localStorage. */
export const draftBySessionAtom = atom<Record<string, string>>(readDrafts())

export const setDraftAtom = atom(
	null,
	(get, set, sessionId: string, text: string) => {
		const prev = get(draftBySessionAtom)
		const next = { ...prev }
		if (text.trim()) next[sessionId] = text
		else delete next[sessionId]
		try {
			localStorage.setItem(DRAFTS_KEY, JSON.stringify(next))
		} catch {
			// Persistence is best-effort.
		}
		set(draftBySessionAtom, next)
	},
)

export interface PendingComment {
	path: string
	line: number
	text: string
}

/** Diff comments accumulated in the review panel, waiting to be sent. */
export const pendingCommentsAtom = atom<PendingComment[]>([])

export interface ComposerInsertRequest {
	text: string
	nonce: number
}

/**
 * Cross-component request to append text to the composer. The nonce guarantees
 * repeat inserts (same text twice) still re-trigger the watcher.
 */
export const composerInsertRequestAtom = atom<ComposerInsertRequest | null>(null)

export const appendComposerTextAtom = atom(null, (get, set, text: string) => {
	const prev = get(composerInsertRequestAtom)
	set(composerInsertRequestAtom, {
		text,
		nonce: (prev?.nonce ?? 0) + 1,
	})
})

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
