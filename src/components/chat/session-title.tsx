import { useAtomValue } from "jotai"
import { getProjectDirectoryLabel } from "@/lib/project-display"
import { sessionTitle } from "@/lib/sessions"
import { activeSessionIdAtom, projectPathAtom, sessionsAtom } from "@/stores/atoms"

export function SessionTitle() {
	const sessions = useAtomValue(sessionsAtom)
	const activeSessionId = useAtomValue(activeSessionIdAtom)
	const projectPath = useAtomValue(projectPathAtom)

	const activeIndex = sessions.findIndex((session) => session.sessionId === activeSessionId)
	const activeSession = activeIndex >= 0 ? sessions[activeIndex] : null

	if (!activeSession) {
		return (
			<span
				data-tauri-drag-region
				className="block min-w-0 flex-1 truncate text-sm text-muted-foreground"
			>
				Sin sesión activa
			</span>
		)
	}

	const directoryLabel = getProjectDirectoryLabel(projectPath)
	const title = sessionTitle(activeSession, activeIndex)

	return (
		<span
			data-tauri-drag-region
			className="block min-w-0 flex-1 truncate text-sm"
		>
			<span className="text-muted-foreground/70">{directoryLabel}/</span>
			<span className="font-medium text-foreground">{title}</span>
		</span>
	)
}