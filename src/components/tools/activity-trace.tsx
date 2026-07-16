import { StructuredToolActivity } from "@/components/tools/structured-tool-activity"
import type { ToolCallState } from "@/types/acp"

interface ActivityTraceProps {
	toolCalls: ToolCallState[]
}

/** Compact in-turn tool activity grouped by files, commands, and other tools. */
export function ActivityTrace({ toolCalls }: ActivityTraceProps) {
	if (toolCalls.length === 0) return null
	return <StructuredToolActivity toolCalls={toolCalls} compact />
}