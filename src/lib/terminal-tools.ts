import type { ChatMessage, ToolCall } from "@/types/acp"

export function isTerminalTool(tool: ToolCall): boolean {
	if (typeof tool.content === "object" && tool.content?.type === "terminal") {
		return true
	}
	return tool.kind === "terminal" || tool.title.toLowerCase().includes("terminal")
}

export function terminalIdFromTool(tool: ToolCall): string | null {
	if (typeof tool.content === "object" && tool.content?.type === "terminal") {
		return tool.content.terminalId
	}
	return null
}

export function collectTerminalTools(messages: ChatMessage[]): ToolCall[] {
	const seen = new Set<string>()
	const tools: ToolCall[] = []
	for (const message of messages) {
		for (const tool of message.toolCalls) {
			if (!isTerminalTool(tool) || seen.has(tool.id)) continue
			seen.add(tool.id)
			tools.push(tool)
		}
	}
	return tools
}
