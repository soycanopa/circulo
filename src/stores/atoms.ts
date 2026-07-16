import { atom } from "jotai"
import { getAppSettings, type AppSettings } from "@/lib/app-settings"
import type { ContextWindowSnapshot } from "@/lib/context-window"
import type {
	AgentCapabilities,
	ChatMessage,
	ConfigOption,
	CredentialRequest,
	PermissionRequest,
	SessionInfo,
	SessionStatus,
	ToolCallState,
} from "@/types/acp"

export const projectPathAtom = atom<string | null>(null)
export const sessionStatusAtom = atom<SessionStatus>("disconnected")
export const promptInFlightAtom = atom(false)
export const replayingHistoryAtom = atom(false)
export const messagesAtom = atom<ChatMessage[]>([])
export const streamingTextAtom = atom<string>("")
export const configOptionsAtom = atom<ConfigOption[]>([])
export const activePermissionAtom = atom<PermissionRequest | null>(null)

/** Credential / auth elicitation awaiting user input in the composer. */
export const activeCredentialAtom = atom<CredentialRequest | null>(null)
export const errorMessageAtom = atom<string | null>(null)
export const sessionsAtom = atom<SessionInfo[]>([])
export const activeSessionIdAtom = atom<string | null>(null)
export const agentCapabilitiesAtom = atom<AgentCapabilities | null>(null)

/** Sentinel value: new thread awaiting project folder selection (no session yet). */
export const NEW_THREAD_PICKER_ID = "__new_thread__"

/** Session id that should show the thread folder picker (set on new thread). */
export const threadFolderPickerSessionIdAtom = atom<string | null>(null)

export interface PendingPlan {
	content: string
	timestamp: number
}

/** Plan markdown awaiting user accept / comment / reject. */
export const pendingPlanAtom = atom<PendingPlan | null>(null)

/** When true, the chat input sends feedback about the pending plan. */
export const planCommentModeAtom = atom(false)

/** Tracks whether the active prompt turn expects a plan preview response. */
export const planTurnActiveAtom = atom(false)

/** Latest context window usage for the active session (from ACP usage_update). */
export const contextWindowAtom = atom<ContextWindowSnapshot | null>(null)

export type ToolOverlayState =
	| { type: "single"; toolCall: ToolCallState }
	| { type: "multi-diff"; toolCalls: ToolCallState[]; activeId?: string }
	| null

/** Fullscreen preview for tool output (C8). */
export const toolOverlayAtom = atom<ToolOverlayState>(null)

export interface PlanOverlayState {
	content: string
	isStreaming?: boolean
	actionsEnabled?: boolean
	onDownload?: () => void
	onAccept?: () => void
	onAcceptAndCompact?: () => void
	onComment?: () => void
	onReject?: () => void
}

export const planOverlayAtom = atom<PlanOverlayState | null>(null)

export const settingsOpenAtom = atom(false)
export const appSettingsAtom = atom<AppSettings>(getAppSettings())

/** Bottom terminal drawer below the chat composer. */
export const terminalOpenAtom = atom(false)

/** Resizable height (px) for the terminal drawer. */
export const terminalHeightAtom = atom(240)

/** Right sidebar for session file diffs (Synara-style review panel). */
export const diffPanelOpenAtom = atom(false)

/** Active tool call id shown in the diff panel. */
export const activeDiffToolIdAtom = atom<string | null>(null)
