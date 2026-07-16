import { useCallback } from "react"
import { ConfigOptionSelector } from "@/components/chat/config-option-selector"
import type { ConfigOption } from "@/types/acp"

function isThinkingOption(option: ConfigOption): boolean {
	const category = option.category?.toLowerCase() ?? ""
	return (
		option.id === "effort" ||
		category.includes("thought") ||
		category.includes("reasoning")
	)
}

export function ThinkingSelector() {
	const match = useCallback((option: ConfigOption) => isThinkingOption(option), [])
	return (
		<ConfigOptionSelector
			match={match}
			placeholder="Thinking"
			maxWidthClass="max-w-32"
		/>
	)
}