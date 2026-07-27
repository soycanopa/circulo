import { atom } from "jotai"
import type {
	AgentCapabilities,
	ChatMessage,
	ConfigOption,
	PermissionRequest,
	SessionStatus,
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
