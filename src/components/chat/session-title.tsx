import { Home } from "lucide-react"
import { useAtomValue } from "jotai"
import { SessionTitleMenu } from "@/components/chat/session-title-menu"
import { useSessions } from "@/hooks/use-sessions"
import { getProjectDirectoryLabel } from "@/lib/project-display"
import { sessionTitle } from "@/lib/sessions"
import { activeSessionIdAtom, projectPathAtom, sessionsAtom } from "@/stores/atoms"

export function SessionTitle() {
	const sessions = useAtomValue(sessionsAtom)
	const activeSessionId = useAtomValue(activeSessionIdAtom)
	const projectPath = useAtomValue(projectPathAtom)
	const { renameSession } = useSessions()

	const activeIndex = sessions.findIndex((session) => session.sessionId === activeSessionId)
	const activeSession = activeIndex >= 0 ? sessions[activeIndex] : null

	if (!activeSession || !activeSessionId) {
		return (
			<span className="block min-w-0 flex-1 truncate text-sm text-muted-foreground">
				Sin sesión activa
			</span>
		)
	}

	const directoryLabel = getProjectDirectoryLabel(projectPath)
	const title = sessionTitle(activeSession, activeIndex)

	return (
		<span className="flex min-w-0 flex-1 items-center text-sm leading-none">
			<span className="inline-flex shrink-0 items-center gap-1 text-muted-foreground/70">
				<Home className="size-3" />
				{directoryLabel}
			</span>
			<span className="mx-1.5 shrink-0 text-muted-foreground/40">/</span>
			<span className="flex min-w-0 items-center gap-1">
				<span className="truncate font-medium text-foreground">{title}</span>
				<SessionTitleMenu
					sessionId={activeSessionId}
					title={title}
					onRename={renameSession}
				/>
			</span>
		</span>
	)
}