import { useAtomValue } from "jotai"
import { sessionTitle } from "@/lib/sessions"
import { activeSessionIdAtom, sessionsAtom } from "@/stores/atoms"

export function SessionTitle() {
	const sessions = useAtomValue(sessionsAtom)
	const activeSessionId = useAtomValue(activeSessionIdAtom)

	const activeIndex = sessions.findIndex((session) => session.sessionId === activeSessionId)
	const activeSession = activeIndex >= 0 ? sessions[activeIndex] : null

	if (!activeSession) {
		return <span className="truncate text-sm text-muted-foreground">Sin sesión activa</span>
	}

	return (
		<span className="truncate text-sm font-medium text-foreground">
			{sessionTitle(activeSession, activeIndex)}
		</span>
	)
}