import type { ChatMessage, ToolCall } from "@/types/acp"

export function isDiffTool(tool: ToolCall): boolean {
	return (
		tool.kind === "diff" ||
		Boolean(tool.content?.startsWith("[diff ")) ||
		tool.title.toLowerCase().includes("diff")
	)
}

export function collectDiffTools(messages: ChatMessage[]): ToolCall[] {
	const seen = new Set<string>()
	const tools: ToolCall[] = []
	for (const message of messages) {
		for (const tool of message.toolCalls) {
			if (!isDiffTool(tool) || seen.has(tool.id)) continue
			seen.add(tool.id)
			tools.push(tool)
		}
	}
	return tools
}
