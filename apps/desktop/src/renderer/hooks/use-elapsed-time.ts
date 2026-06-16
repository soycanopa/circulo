/**
 * Hook that returns a live-updating elapsed time string for running tool calls.
 *
 * Ticks every second while the tool is running/pending, returning a formatted
 * duration like "3s", "1m 23s". Returns `undefined` when the tool is not active.
 *
 * Uses the client-side "first seen" timestamp when available (matching the
 * wall-clock behavior of `getToolDuration` for completed tools), falling back
 * to the server-side `time.start`.
 */

import { useEffect, useState } from "react"
import { getPartFirstSeenAt } from "../atoms/parts"
import type { ToolPart } from "../lib/types"

function formatElapsed(ms: number): string {
	if (ms < 1000) return "0s"
	const seconds = Math.floor(ms / 1000)
	if (seconds < 60) return `${seconds}s`
	const minutes = Math.floor(seconds / 60)
	const remainingSeconds = seconds % 60
	return `${minutes}m ${remainingSeconds}s`
}

export function useToolElapsedTime(part: ToolPart): string | undefined {
	const status = part.state.status
	const isActive = status === "running" || status === "pending"

	// Determine the start time: prefer client-side first-seen, fall back to server time
	const firstSeen = getPartFirstSeenAt(part.id)
	const serverStart = "time" in part.state ? (part.state.time as { start: number }).start : undefined
	const startTime = firstSeen ?? serverStart

	const [elapsed, setElapsed] = useState<string | undefined>(() => {
		if (!isActive || !startTime) return undefined
		return formatElapsed(Date.now() - startTime)
	})

	useEffect(() => {
		if (!isActive || !startTime) {
			setElapsed(undefined)
			return
		}

		// Compute immediately
		setElapsed(formatElapsed(Date.now() - startTime))

		const intervalId = setInterval(() => {
			setElapsed(formatElapsed(Date.now() - startTime))
		}, 1_000)

		return () => clearInterval(intervalId)
	}, [isActive, startTime])

	return elapsed
}
