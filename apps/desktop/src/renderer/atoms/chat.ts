import { atom } from "jotai"
import { atomFamily } from "jotai-family"
import type { ModelRef } from "../hooks/use-opencode-data"
import { appStore } from "./store"

// ============================================================
// Session Settings (Atomic per session)
// ============================================================

export interface SessionChatSettings {
	selectedModel: ModelRef | null
	selectedAgent: string | null
	selectedVariant: string | undefined
}

/**
 * Persisted chat settings per session.
 * Seeded from project preferences, but overrides are session-specific.
 */
export const sessionSettingsAtom = atomFamily((_sessionId: string) =>
	atom<SessionChatSettings>({
		selectedModel: null,
		selectedAgent: null,
		selectedVariant: undefined,
	}),
)

/**
 * Directory to auto-select when entering the new-chat view.
 * Set by the sidebar Chat button to bypass project selection.
 * Cleared after consumption by NewChat.
 */
export const chatInitDirectoryAtom = atom<string | null>(null)

/**
 * Global timer for live-updating UI (durations, elapsed time).
 * Updates once per second.
 */
export const nowAtom = atom(Date.now())

if (typeof window !== "undefined") {
	setInterval(() => {
		appStore.set(nowAtom, Date.now())
	}, 1000)
}
