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
