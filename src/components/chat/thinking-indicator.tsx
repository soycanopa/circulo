import { Loader2 } from "lucide-react"
import { useThinkingPhrase } from "@/hooks/use-thinking-phrase"

interface ThinkingIndicatorProps {
	active: boolean
}

export function ThinkingIndicator({ active }: ThinkingIndicatorProps) {
	const phrase = useThinkingPhrase(active)

	if (!active) return null

	return (
		<div className="flex max-w-full items-center gap-2.5 rounded-xl border border-[#3B5EF9]/25 bg-[#3B5EF9]/8 px-4 py-3 text-sm">
			<Loader2 className="size-4 shrink-0 animate-spin text-[#3B5EF9]" />
			<span className="text-foreground/90 transition-opacity duration-300">{phrase}</span>
		</div>
	)
}