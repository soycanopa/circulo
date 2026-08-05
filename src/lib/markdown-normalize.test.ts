import { describe, expect, it } from "vitest"
import {
	normalizeAgentMarkdown,
	prepareMarkdownForRender,
	stripIncompleteMarkdownDelimiters,
} from "@/lib/markdown-normalize"

describe("normalizeAgentMarkdown", () => {
	it("converts pseudo-list lines with em-dash to bullet + bold title", () => {
		const input =
			"**Qwen2.5-1.5B-Instruct — la mejor relación\n**Qwen2.5-0.5B-Instruct — ~mitad de RAM"
		const result = normalizeAgentMarkdown(input)
		expect(result).toBe(
			"- **Qwen2.5-1.5B-Instruct** — la mejor relación\n- **Qwen2.5-0.5B-Instruct** — ~mitad de RAM",
		)
	})

	it("closes bold on lines without em-dash separator", () => {
		const result = normalizeAgentMarkdown("**solo titulo sin dash")
		expect(result).toBe("- **solo titulo sin dash**")
	})

	it("does not alter properly closed inline bold", () => {
		const input = "El modelo **Llama-3.2-1B-Instruct-8bit** usa RAM"
		expect(normalizeAgentMarkdown(input)).toBe(input)
	})

	it("normalizes unicode asterisks to ASCII", () => {
		const result = normalizeAgentMarkdown("＊＊unicode bold＊＊")
		expect(result).toBe("**unicode bold**")
	})

	it("skips malformed bold lines inside fenced code blocks", () => {
		const input = "```\n**not a list\n**also not\n```"
		expect(normalizeAgentMarkdown(input)).toBe(input)
	})
})

describe("stripIncompleteMarkdownDelimiters", () => {
	it("removes trailing unclosed **", () => {
		expect(stripIncompleteMarkdownDelimiters("partial **bol")).toBe("partial bol")
	})

	it("removes trailing unclosed __", () => {
		expect(stripIncompleteMarkdownDelimiters("partial __bol")).toBe("partial bol")
	})

	it("removes trailing unclosed backtick", () => {
		expect(stripIncompleteMarkdownDelimiters("use `code")).toBe("use code")
	})

	it("keeps complete emphasis intact", () => {
		const input = "text **bold** more"
		expect(stripIncompleteMarkdownDelimiters(input)).toBe(input)
	})
})

describe("prepareMarkdownForRender", () => {
	it("applies normalization always", () => {
		const result = prepareMarkdownForRender("**Model — desc")
		expect(result).toBe("- **Model** — desc")
	})

	it("strips incomplete delimiters only when streaming", () => {
		expect(prepareMarkdownForRender("hello **par", false)).toBe("hello **par")
		expect(prepareMarkdownForRender("hello **par", true)).toBe("hello par")
	})
})
