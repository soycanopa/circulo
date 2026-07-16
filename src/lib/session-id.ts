import { isOptimisticSessionId } from "@/lib/optimistic-session"

const PLACEHOLDER_SESSION_ID = "pending"

export function isPlaceholderSessionId(sessionId: string | null | undefined): boolean {
	return !sessionId || sessionId === PLACEHOLDER_SESSION_ID
}

export function normalizeSessionId(sessionId: string | null | undefined): string | null {
	if (isPlaceholderSessionId(sessionId)) return null
	if (isOptimisticSessionId(sessionId)) return null
	return sessionId!
}