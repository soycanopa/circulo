import { Home } from "lucide-react"
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
			<div className="flex min-w-0 flex-1 flex-col justify-center gap-0.5 leading-tight">
				<p className="truncate text-sm text-muted-foreground">Sin sesión activa</p>
			</div>
		)
	}

	const directoryLabel = getProjectDirectoryLabel(projectPath)
	const title = sessionTitle(activeSession, activeIndex)
	const sessionLabel = `Sesión ${activeIndex + 1}`

	return (
		<div className="flex min-w-0 flex-1 flex-col justify-center gap-0.5 leading-tight">
			<p className="truncate text-sm font-medium text-foreground">{title}</p>
			<p className="flex min-w-0 items-center truncate text-xs text-muted-foreground/70">
				<Home className="mr-1 size-3 shrink-0" />
				<span className="truncate">
					{directoryLabel}
					<span className="mx-1 text-muted-foreground/40">/</span>
					{sessionLabel}
				</span>
			</p>
		</div>
	)
}