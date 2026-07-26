import type { ChatMessage } from "@/types/acp"

const cache = new Map<string, ChatMessage[]>()

export function getCachedSessionMessages(sessionId: string): ChatMessage[] | null {
	const cached = cache.get(sessionId)
	return cached ? structuredClone(cached) : null
}

export function cacheSessionMessages(sessionId: string, messages: ChatMessage[]): void {
	if (!sessionId || messages.length === 0) {
		cache.delete(sessionId)
		return
	}
	cache.set(sessionId, structuredClone(messages))
}

export function clearCachedSessionMessages(sessionId: string): void {
	cache.delete(sessionId)
}

export function clearSessionMessagesCache(): void {
	cache.clear()
}
