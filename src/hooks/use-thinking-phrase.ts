import { useEffect, useState } from "react"
import { pickThinkingPhrase } from "@/lib/thinking-phrases"
import type { TurnPhase } from "@/lib/turn-phase"

const ROTATE_MS = 2800

export function useThinkingPhrase(active: boolean, phase: TurnPhase = "pending") {
	const [phrase, setPhrase] = useState(() => pickThinkingPhrase(undefined, phase))

	useEffect(() => {
		if (!active) return
		setPhrase(pickThinkingPhrase(undefined, phase))
		const interval = window.setInterval(() => {
			setPhrase((current) => pickThinkingPhrase(current, phase))
		}, ROTATE_MS)
		return () => window.clearInterval(interval)
	}, [active, phase])

	return phrase
}