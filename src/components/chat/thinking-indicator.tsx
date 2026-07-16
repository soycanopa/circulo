import { useThinkingPhrase } from "@/hooks/use-thinking-phrase"
import type { TurnPhase } from "@/lib/turn-phase"

interface ThinkingIndicatorProps {
	active: boolean
	phase?: TurnPhase
}

export function ThinkingIndicator({ active, phase = "pending" }: ThinkingIndicatorProps) {
	const phrase = useThinkingPhrase(active, phase)

	if (!active) return null

	return (
		<p key={phrase} className="thinking-shimmer max-w-full text-sm">
			{phrase}
		</p>
	)
}