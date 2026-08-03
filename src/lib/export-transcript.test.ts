import { describe, expect, it } from "vitest"
import { formatTranscriptMarkdown } from "@/lib/export-transcript"
import type { ChatMessage } from "@/types/acp"

describe("formatTranscriptMarkdown", () => {
	it("formats user and assistant messages with tool blocks", () => {
		const messages: ChatMessage[] = [
			{
				id: "u1",
				role: "user",
				content: "Hello",
				timestamp: 1,
				toolCalls: [],
			},
			{
				id: "a1",
				role: "assistant",
				content: "Hi there",
				timestamp: 2,
				toolCalls: [
					{
						id: "t1",
						title: "Read file",
						status: "completed",
						kind: "other",
						content: "file contents",
					},
				],
			},
		]

		const md = formatTranscriptMarkdown("My chat", messages)
		expect(md).toContain("# My chat")
		expect(md).toContain("## User")
		expect(md).toContain("Hello")
		expect(md).toContain("## Assistant")
		expect(md).toContain("Hi there")
		expect(md).toContain("### Tool: Read file (completed)")
		expect(md).toContain("file contents")
	})
})
