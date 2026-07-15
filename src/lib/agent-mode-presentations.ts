import type { LucideIcon } from "lucide-react"
import { ClipboardList, Hand, Info, ShieldCheck } from "lucide-react"

export interface AgentModePresentation {
	values: string[]
	title: string
	description: string
	icon: LucideIcon
}

export const AGENT_MODE_PRESENTATIONS: AgentModePresentation[] = [
	{
		values: ["ask", "ask_before_edits", "ask-before-edits"],
		title: "Ask before changes",
		description: "Ask before file changes.",
		icon: Hand,
	},
	{
		values: ["auto", "default", "edit_automatically", "edit-automatically", "build"],
		title: "Edit automatically",
		description: "Edit files automatically.",
		icon: ShieldCheck,
	},
	{
		values: ["plan", "plan_mode", "plan-mode"],
		title: "Plan mode",
		description: "Plan before editing.",
		icon: ClipboardList,
	},
	{
		values: ["full", "yolo", "full_access", "full-access"],
		title: "Full access",
		description: "Run with fewer confirmations.",
		icon: Info,
	},
]

export function resolveAgentModePresentation(
	value: string,
	name?: string,
	description?: string,
): AgentModePresentation & { value: string } {
	const normalized = value.trim().toLowerCase()
	const match = AGENT_MODE_PRESENTATIONS.find((entry) =>
		entry.values.some(
			(candidate) =>
				normalized === candidate ||
				normalized.includes(candidate) ||
				candidate.includes(normalized),
		),
	)

	if (match) return { ...match, value }

	return {
		value,
		values: [value],
		title: name?.trim() || value,
		description: description?.trim() || "",
		icon: ShieldCheck,
	}
}

export function isPlanModeValue(value: string | undefined): boolean {
	if (!value) return false
	const normalized = value.trim().toLowerCase()
	return AGENT_MODE_PRESENTATIONS[2].values.some(
		(candidate) =>
			normalized === candidate ||
			normalized.includes("plan") ||
			candidate.includes(normalized),
	)
}