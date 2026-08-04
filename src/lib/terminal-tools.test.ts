import { describe, expect, it } from "vitest"
import { collectTerminalTools, isTerminalTool, terminalIdFromTool } from "@/lib/terminal-tools"
import type { ToolCall } from "@/types/acp"

const terminalTool: ToolCall = {
	id: "t1",
	title: "Run tests",
	status: "completed",
	kind: "terminal",
	content: { type: "terminal", terminalId: "term_abc" },
}

describe("terminal-tools", () => {
	it("detects terminal tool content", () => {
		expect(isTerminalTool(terminalTool)).toBe(true)
		expect(terminalIdFromTool(terminalTool)).toBe("term_abc")
	})

	it("collects terminal tools from messages", () => {
		const tools = collectTerminalTools([
			{
				id: "m1",
				role: "assistant",
				content: "",
				timestamp: 1,
				toolCalls: [terminalTool],
			},
		])
		expect(tools).toHaveLength(1)
	})
})
