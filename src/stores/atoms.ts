import { atom } from "jotai"
import type { ChatMessage, ConfigOption, PermissionRequest, SessionStatus } from "@/types/acp"

export const projectPathAtom = atom<string | null>(null)
export const sessionStatusAtom = atom<SessionStatus>("disconnected")
export const messagesAtom = atom<ChatMessage[]>([])
export const streamingTextAtom = atom<string>("")
export const configOptionsAtom = atom<ConfigOption[]>([])
export const activePermissionAtom = atom<PermissionRequest | null>(null)
export const errorMessageAtom = atom<string | null>(null)