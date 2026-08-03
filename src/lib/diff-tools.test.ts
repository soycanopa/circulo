import { describe, expect, it } from "vitest"
import { collectDiffTools, isDiffTool } from "@/lib/diff-tools"
import type { ChatMessage, ToolCall } from "@/types/acp"

function tool(overrides: Partial<ToolCall> = {}): ToolCall {
	return {
		id: "t1",
		title: "Edit file",
		status: "completed",
		kind: "other",
		content: "plain output",
		...overrides,
	}
}

describe("isDiffTool", () => {
	it("detects diff kind", () => {
		expect(isDiffTool(tool({ kind: "diff" }))).toBe(true)
	})

	it("detects structured diff content", () => {
		expect(
			isDiffTool(
				tool({
					content: {
						type: "diff",
						path: "src/a.ts",
						oldText: "a",
						newText: "b",
					},
				}),
			),
		).toBe(true)
	})

	it("detects diff title fallback", () => {
		expect(isDiffTool(tool({ title: "Apply diff to App.tsx" }))).toBe(true)
	})
})

describe("collectDiffTools", () => {
	it("collects unique diff tools across messages", () => {
		const messages: ChatMessage[] = [
			{
				id: "m1",
				role: "assistant",
				content: "",
				timestamp: 1,
				toolCalls: [
					tool({ id: "d1", kind: "diff", title: "Diff 1" }),
					tool({ id: "d2", title: "Write diff" }),
				],
			},
			{
				id: "m2",
				role: "assistant",
				content: "",
				timestamp: 2,
				toolCalls: [tool({ id: "d1", kind: "diff", title: "Diff 1 dup" })],
			},
		]

		const diffs = collectDiffTools(messages)
		expect(diffs.map((d) => d.id)).toEqual(["d1", "d2"])
	})
})
