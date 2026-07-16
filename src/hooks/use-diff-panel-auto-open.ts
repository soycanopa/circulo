import { useAtomValue, useSetAtom } from "jotai"
import { useEffect, useRef } from "react"
import { getLatestSessionDiff } from "@/lib/session-diffs"
import {
	activeDiffToolIdAtom,
	diffPanelOpenAtom,
	messagesAtom,
	promptInFlightAtom,
} from "@/stores/atoms"

/** Opens the diff panel when the agent produces a new file diff during an active turn. */
export function useDiffPanelAutoOpen() {
	const messages = useAtomValue(messagesAtom)
	const promptInFlight = useAtomValue(promptInFlightAtom)
	const setDiffPanelOpen = useSetAtom(diffPanelOpenAtom)
	const setActiveDiffToolId = useSetAtom(activeDiffToolIdAtom)
	const lastDiffIdRef = useRef<string | null>(null)

	useEffect(() => {
		if (!promptInFlight) return
		const latest = getLatestSessionDiff(messages)
		if (!latest || latest.id === lastDiffIdRef.current) return
		lastDiffIdRef.current = latest.id
		setActiveDiffToolId(latest.id)
		setDiffPanelOpen(true)
	}, [messages, promptInFlight, setActiveDiffToolId, setDiffPanelOpen])
}