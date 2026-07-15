const ARCHIVED_SESSIONS_KEY = "forge-archived-sessions"

export function getArchivedSessionIds(): string[] {
	try {
		const raw = localStorage.getItem(ARCHIVED_SESSIONS_KEY)
		if (!raw) return []
		const parsed = JSON.parse(raw) as unknown
		return Array.isArray(parsed) ? parsed.filter((v) => typeof v === "string") : []
	} catch {
		return []
	}
}

export function isArchivedSession(sessionId: string): boolean {
	return getArchivedSessionIds().includes(sessionId)
}

export function archiveSessionId(sessionId: string): string[] {
	const current = getArchivedSessionIds()
	if (current.includes(sessionId)) return current
	const next = [...current, sessionId]
	localStorage.setItem(ARCHIVED_SESSIONS_KEY, JSON.stringify(next))
	return next
}

export function unarchiveSessionId(sessionId: string): string[] {
	const next = getArchivedSessionIds().filter((id) => id !== sessionId)
	localStorage.setItem(ARCHIVED_SESSIONS_KEY, JSON.stringify(next))
	return next
}

export function removeArchivedSessionId(sessionId: string): string[] {
	return unarchiveSessionId(sessionId)
}