import { describe, expect, it } from "vitest"
import { parseMarkdownBlocks } from "@/lib/simple-markdown"

describe("parseMarkdownBlocks", () => {
	it("parses fenced code blocks", () => {
		const blocks = parseMarkdownBlocks("before\n```ts\nconst x = 1\n```\nafter")
		expect(blocks).toEqual([
			{ type: "paragraph", text: "before" },
			{ type: "code", lang: "ts", text: "const x = 1" },
			{ type: "paragraph", text: "after" },
		])
	})

	it("parses headings", () => {
		const blocks = parseMarkdownBlocks("# Title\n\nBody")
		expect(blocks[0]).toEqual({ type: "heading", level: 1, text: "Title" })
		expect(blocks[1]).toEqual({ type: "paragraph", text: "Body" })
	})

	it("parses bullet lists", () => {
		const blocks = parseMarkdownBlocks("- one\n- two")
		expect(blocks[0]).toEqual({
			type: "list",
			ordered: false,
			items: ["one", "two"],
		})
	})
})
