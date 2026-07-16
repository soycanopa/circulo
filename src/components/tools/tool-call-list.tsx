import { StructuredToolActivity } from "@/components/tools/structured-tool-activity"
import type { ToolCallState } from "@/types/acp"

interface ToolCallListProps {
	toolCalls: ToolCallState[]
}

export function ToolCallList({ toolCalls }: ToolCallListProps) {
	if (toolCalls.length === 0) return null
	return <StructuredToolActivity toolCalls={toolCalls} />
}