import type { SessionInfo } from "@/types/acp"

const MAX_DERIVED_TITLE = 48

export function deriveTitleFromMessage(text: string): string {
	const cleaned = text.replace(/\s+/g, " ").trim()
	if (!cleaned) return "Nueva sesión"
	if (cleaned.length <= MAX_DERIVED_TITLE) return cleaned
	return `${cleaned.slice(0, MAX_DERIVED_TITLE - 1)}…`
}

export function sessionTitle(session: SessionInfo, index = 0): string {
	const trimmed = session.title?.trim()
	if (trimmed) return trimmed
	return `Sesión ${index + 1}`
}