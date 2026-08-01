import { beforeEach, describe, expect, it, vi } from "vitest"
import {
	appendStreamToMessages,
	applySessionUpdate,
	extractTextFromContent,
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
