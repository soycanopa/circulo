import { ToolCallGroupCard } from "@/components/tools/tool-call-group"
import { groupToolCalls } from "@/lib/tool-call-groups"
import type { ToolCallState } from "@/types/acp"

interface ToolCallListProps {
	toolCalls: ToolCallState[]
}

export function ToolCallList({ toolCalls }: ToolCallListProps) {
	if (toolCalls.length === 0) return null

	const groups = groupToolCalls(toolCalls)

	return (
		<>
			{groups.map((group) => (
				<ToolCallGroupCard key={group.key} group={group} />
			))}
		</>
	)
}