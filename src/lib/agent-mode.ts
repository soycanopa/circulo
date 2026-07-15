import { isPlanModeValue } from "@/lib/agent-mode-presentations"
import type { ConfigOption } from "@/types/acp"

export function findModeOption(options: ConfigOption[]): ConfigOption | undefined {
	return options.find(
		(option) =>
			option.id === "mode" || (option.category?.toLowerCase() ?? "") === "mode",
	)
}

export function isAgentPlanMode(options: ConfigOption[]): boolean {
	return isPlanModeValue(findModeOption(options)?.currentValue)
}