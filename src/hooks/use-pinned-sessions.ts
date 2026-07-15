import { useCallback, useEffect, useState } from "react"
import {
	getPinnedSessionIds,
	setPinnedSessionIds,
	togglePinnedSession,
} from "@/lib/pinned-sessions"

export function usePinnedSessions() {
	const [pinnedIds, setPinnedIds] = useState<string[]>(() => getPinnedSessionIds())

	useEffect(() => {
		const onStorage = (event: StorageEvent) => {
			if (event.key === "forge-pinned-sessions") {
				setPinnedIds(getPinnedSessionIds())
			}
		}
		window.addEventListener("storage", onStorage)
		return () => window.removeEventListener("storage", onStorage)
	}, [])

	const togglePin = useCallback((sessionId: string) => {
		const next = togglePinnedSession(sessionId)
		setPinnedIds(next)
		return next
	}, [])

	const isPinned = useCallback((sessionId: string) => pinnedIds.includes(sessionId), [pinnedIds])

	const reorderPinned = useCallback((ids: string[]) => {
		setPinnedSessionIds(ids)
		setPinnedIds(ids)
	}, [])

	return { pinnedIds, togglePin, isPinned, reorderPinned }
}