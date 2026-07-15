import { useCallback } from "react"
import { ConfigOptionSelector } from "@/components/chat/config-option-selector"
import type { ConfigOption } from "@/types/acp"

function isModeOption(option: ConfigOption): boolean {
	const category = option.category?.toLowerCase() ?? ""
	return option.id === "mode" || category === "mode"
}

export function AgentModeSelector() {
	const match = useCallback((option: ConfigOption) => isModeOption(option), [])
	return (
		<ConfigOptionSelector
			match={match}
			placeholder="Modo"
			fallbackLabel="Plan"
			alwaysVisible
			maxWidthClass="max-w-32"
		/>
	)
}