import type { Store } from "jotai/vanilla/store"
import { loadChatTranscript } from "@/lib/tauri"
import { sessionsAtom, type SessionUiState } from "@/stores/atoms"
import type { ChatMessage } from "@/types/acp"

const EMPTY_SESSION: SessionUiState = {
	messages: [],
	streaming: "",
	promptInFlight: false,
	status: "idle",
	configOptions: [],
	contextUsage: null,
}

export function getLiveSessionMessages(
	store: Store,
	sessionId: string,
): ChatMessage[] {
	return store.get(sessionsAtom)[sessionId]?.messages ?? []
}

export async function transcriptHasMessages(
	projectPath: string,
	sessionId: string,
): Promise<boolean> {
	try {
		const transcript = await loadChatTranscript(projectPath, sessionId)
		return transcript.messages.length > 0
	} catch {
		return false
	}
}

/** True only when the live buffer and on-disk transcript are both empty. */
export async function isLiveSessionPristine(
	store: Store,
	projectPath: string,
	sessionId: string,
): Promise<boolean> {
	if (getLiveSessionMessages(store, sessionId).length > 0) {
		return false
	}
	return !(await transcriptHasMessages(projectPath, sessionId))
}

/** Load transcript messages into the live session slot when the buffer is empty. */
export async function hydrateSessionFromDisk(
	store: Store,
	projectPath: string,
	sessionId: string,
): Promise<ChatMessage[]> {
	const existing = getLiveSessionMessages(store, sessionId)
	if (existing.length > 0) return existing

	try {
		const transcript = await loadChatTranscript(projectPath, sessionId)
		if (transcript.messages.length === 0) return []

		store.set(sessionsAtom, (prev) => {
			const current = prev[sessionId] ?? { ...EMPTY_SESSION }
			return {
				...prev,
				[sessionId]: {
					...current,
					messages: transcript.messages,
				},
			}
		})
		return transcript.messages
	} catch {
		return []
	}
}
