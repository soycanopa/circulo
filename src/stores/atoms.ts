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
export const selectedDiffToolAtom = atom<ToolCall | null>(null)

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
