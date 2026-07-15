import { useEffect, useState } from "react"
import { pickThinkingPhrase } from "@/lib/thinking-phrases"

const ROTATE_MS = 2800

export function useThinkingPhrase(active: boolean) {
	const [phrase, setPhrase] = useState(() => pickThinkingPhrase())

	useEffect(() => {
		if (!active) return
		setPhrase(pickThinkingPhrase())
		const interval = window.setInterval(() => {
			setPhrase((current) => pickThinkingPhrase(current))
		}, ROTATE_MS)
		return () => window.clearInterval(interval)
	}, [active])

	return phrase
}