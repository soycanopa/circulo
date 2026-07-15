import { useCallback, useEffect, useState } from "react"
import {
	archiveSessionId,
	getArchivedSessionIds,
	isArchivedSession,
	unarchiveSessionId,
} from "@/lib/archived-sessions"

export function useArchivedSessions() {
	const [archivedIds, setArchivedIds] = useState<string[]>(() => getArchivedSessionIds())

	useEffect(() => {
		const onStorage = (event: StorageEvent) => {
			if (event.key === "circulo-archived-sessions") {
				setArchivedIds(getArchivedSessionIds())
			}
		}
		window.addEventListener("storage", onStorage)
		return () => window.removeEventListener("storage", onStorage)
	}, [])

	const archive = useCallback((sessionId: string) => {
		const next = archiveSessionId(sessionId)
		setArchivedIds(next)
		return next
	}, [])

	const unarchive = useCallback((sessionId: string) => {
		const next = unarchiveSessionId(sessionId)
		setArchivedIds(next)
		return next
	}, [])

	const isArchived = useCallback(
		(sessionId: string) => archivedIds.includes(sessionId),
		[archivedIds],
	)

	return { archivedIds, archive, unarchive, isArchived }
}

export { isArchivedSession }