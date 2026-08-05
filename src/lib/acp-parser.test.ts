import { beforeEach, describe, expect, it, vi } from "vitest"
import {
	appendStreamToMessages,
	applySessionUpdate,
	extractTextFromContent,
	isQuestionToolCall,
	isTaskToolCall,
	parseUsageUpdate,
	taskStateFromStatus,
} from "@/lib/acp-parser"
import type { ChatMessage } from "@/types/acp"

beforeEach(() => {
	vi.stubGlobal("crypto", {
		randomUUID: () => "test-uuid",
	})
})

describe("extractTextFromContent", () => {
	it("reads plain string content", () => {
		expect(extractTextFromContent("hello")).toBe("hello")
	})

	it("reads ACP text blocks", () => {
		expect(extractTextFromContent({ type: "text", text: "hi" })).toBe("hi")
	})

	it("joins array content", () => {
		expect(
			extractTextFromContent([
				{ type: "text", text: "a" },
				{ type: "text", text: "b" },
			]),
		).toBe("ab")
	})
})

describe("applySessionUpdate", () => {
	const base: ChatMessage[] = [
		{
			id: "u1",
			role: "user",
			content: "Hi",
			toolCalls: [],
			timestamp: 1,
		},
		{
			id: "a1",
			role: "assistant",
			content: "Hello",
			toolCalls: [],
			timestamp: 2,
		},
	]

	it("appends agent_message_chunk to the last assistant bubble", () => {
		const result = applySessionUpdate(base, "", {
			update: {
				sessionUpdate: "agent_message_chunk",
				content: { type: "text", text: " world" },
			},
		})
		expect(result.didStream).toBe(true)
		expect(result.messages[1]?.content).toBe("Hello world")
	})

	it("accepts cumulative chunks without duplicating text", () => {
		const result = applySessionUpdate(base, "", {
			update: {
				sessionUpdate: "agent_message_chunk",
				content: { type: "text", text: "Hello world" },
			},
		})
		expect(result.messages[1]?.content).toBe("Hello world")
	})

	it("merges tool_call updates by id", () => {
		const withTool: ChatMessage[] = [
			...base,
			{
				id: "a2",
				role: "assistant",
				content: "",
				toolCalls: [
					{
						id: "tool-1",
						title: "read",
						status: "pending",
						content: "",
					},
				],
				timestamp: 3,
			},
		]
		const result = applySessionUpdate(withTool, "", {
			update: {
				sessionUpdate: "tool_call_update",
				toolCallId: "tool-1",
				title: "read",
				status: "completed",
				content: "file contents",
			},
		})
		const tools = result.messages[result.messages.length - 1]?.toolCalls ?? []
		expect(tools).toHaveLength(1)
		expect(tools[0]?.status).toBe("completed")
		expect(tools[0]?.content).toBe("file contents")
	})

	it("streams post-tool text after the tool bubble", () => {
		const withTool: ChatMessage[] = [
			{
				id: "u1",
				role: "user",
				content: "read file",
				toolCalls: [],
				timestamp: 1,
			},
			{
				id: "a1",
				role: "assistant",
				content: "Let me check.",
				toolCalls: [
					{
						id: "tool-1",
						title: "read",
						status: "completed",
						content: "",
					},
				],
				timestamp: 2,
			},
		]
		const result = applySessionUpdate(withTool, "", {
			update: {
				sessionUpdate: "agent_message_chunk",
				content: { type: "text", text: "Here is the summary." },
			},
		})
		expect(result.messages).toHaveLength(3)
		expect(result.messages[1]?.toolCalls).toHaveLength(1)
		expect(result.messages[2]?.content).toBe("Here is the summary.")
		expect(result.messages[2]?.toolCalls).toHaveLength(0)
	})
})

describe("appendStreamToMessages", () => {
	it("merges trailing stream buffer into the assistant message", () => {
		const messages: ChatMessage[] = [
			{
				id: "a1",
				role: "assistant",
				content: "Hi",
				toolCalls: [],
				timestamp: 1,
			},
		]
		const next = appendStreamToMessages(messages, " there")
		expect(next[0]?.content).toBe("Hi there")
	})
})

describe("parseUsageUpdate", () => {
	it("reads ACP usage_update payloads", () => {
		expect(
			parseUsageUpdate({
				update: {
					sessionUpdate: "usage_update",
					used: 53_000,
					size: 200_000,
				},
			}),
		).toEqual({ used: 53_000, size: 200_000 })
	})

	it("ignores non-usage updates", () => {
		expect(
			parseUsageUpdate({
				update: { sessionUpdate: "agent_message_chunk", content: "hi" },
			}),
		).toBeNull()
	})
})

describe("isTaskToolCall", () => {
	it("detects task kind from title", () => {
		expect(
			isTaskToolCall({
				title: "Run task",
				kind: "task",
				rawInput: { task: "investigate bug", cwd: "/proj" },
			}),
		).toBe(true)
	})

	it("detects subagent naming", () => {
		expect(isTaskToolCall({ title: "Start subagent", kind: "custom" })).toBe(true)
	})

	it("detects rawInput task field", () => {
		expect(
			isTaskToolCall({
				title: "t",
				kind: "other",
				rawInput: { task: "write tests" },
			}),
		).toBe(true)
	})

	it("requires an agent hint when only cwd is present", () => {
		expect(
			isTaskToolCall({ title: "bash", kind: "shell", rawInput: { cwd: "/proj" } }),
		).toBe(false)
		expect(
			isTaskToolCall({
				title: "agent start",
				kind: "shell",
				rawInput: { cwd: "/proj" },
			}),
		).toBe(true)
	})

	it("ignores unrelated tools", () => {
		expect(
			isTaskToolCall({ title: "Edit file", kind: "edit", rawInput: { filePath: "a.ts" } }),
		).toBe(false)
	})
})

describe("taskStateFromStatus", () => {
	it("maps ACP statuses to task states", () => {
		expect(taskStateFromStatus("pending")).toBe("pending")
		expect(taskStateFromStatus("running")).toBe("running")
		expect(taskStateFromStatus("completed")).toBe("completed")
		expect(taskStateFromStatus("done")).toBe("completed")
		expect(taskStateFromStatus("failed")).toBe("failed")
		expect(taskStateFromStatus("error")).toBe("failed")
	})
})

describe("isQuestionToolCall", () => {
	it("detects question kind from title", () => {
		expect(isQuestionToolCall({ title: "Ask user a question", kind: "custom" })).toBe(
			true,
		)
	})

	it("detects radio/checkbox/text rawInput", () => {
		expect(
			isQuestionToolCall({
				title: "t",
				kind: "other",
				rawInput: { type: "radio", question: "Pick one", options: ["a", "b"] },
			}),
		).toBe(true)
		expect(
			isQuestionToolCall({
				title: "t",
				kind: "other",
				rawInput: { type: "checkbox", question: "Pick many", options: [] },
			}),
		).toBe(true)
		expect(
			isQuestionToolCall({
				title: "t",
				kind: "other",
				rawInput: { type: "text", prompt: "Type it" },
			}),
		).toBe(true)
	})

	it("ignores unrelated tools", () => {
		expect(
			isQuestionToolCall({ title: "Edit file", kind: "edit", rawInput: { filePath: "a.ts" } }),
		).toBe(false)
	})
})
