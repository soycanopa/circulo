import { atom } from "jotai"
import type {
	AgentCapabilities,
	ChatMessage,
	ConfigOption,
	PermissionRequest,
	SessionInfo,
	SessionStatus,
} from "@/types/acp"

export const projectPathAtom = atom<string | null>(null)
export const sessionStatusAtom = atom<SessionStatus>("disconnected")
export const promptInFlightAtom = atom(false)
export const replayingHistoryAtom = atom(false)
export const messagesAtom = atom<ChatMessage[]>([])
export const streamingTextAtom = atom<string>("")
export const configOptionsAtom = atom<ConfigOption[]>([])
export const activePermissionAtom = atom<PermissionRequest | null>(null)
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
