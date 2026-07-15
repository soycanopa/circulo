const PINNED_KEY = "circulo-pinned-sessions"

export function getPinnedSessionIds(): string[] {
	try {
		const raw = localStorage.getItem(PINNED_KEY)
		if (!raw) return []
		const parsed = JSON.parse(raw) as unknown
		return Array.isArray(parsed) ? parsed.filter((id) => typeof id === "string") : []
	} catch {
		return []
	}
}

export function setPinnedSessionIds(ids: string[]): void {
	localStorage.setItem(PINNED_KEY, JSON.stringify(ids))
}

export function unpinSessionId(sessionId: string): string[] {
	const next = getPinnedSessionIds().filter((id) => id !== sessionId)
	setPinnedSessionIds(next)
	return next
}

export function togglePinnedSession(sessionId: string): string[] {
	const current = getPinnedSessionIds()
	const next = current.includes(sessionId)
		? current.filter((id) => id !== sessionId)
		: [...current, sessionId]
	setPinnedSessionIds(next)
	return next
}