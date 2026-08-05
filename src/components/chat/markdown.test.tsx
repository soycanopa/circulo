import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it } from "vitest"
import { Markdown } from "@/components/chat/markdown"

function renderMarkdown(text: string): string {
	return renderToStaticMarkup(<Markdown text={text} />)
}

describe("Markdown", () => {
	it("renders GFM tables", () => {
		const html = renderMarkdown("| A | B |\n|---|---|\n| 1 | 2 |")
		expect(html).toContain("<table")
		expect(html).toContain(">1<")
		expect(html).toContain(">2<")
	})

	it("renders task lists with disabled checkboxes", () => {
		const html = renderMarkdown("- [x] done\n- [ ] todo")
		expect(html).toContain('type="checkbox"')
		expect(html).toContain('checked=""')
		expect(html).toContain('disabled=""')
	})

	it("renders mermaid blocks as code fallback in SSR", () => {
		// In a non-DOM render the lazy mermaid import never runs; the block
		// falls back to the raw code text.
		const html = renderMarkdown("```mermaid\nflowchart LR\nA-->B\n```")
		expect(html).toContain("flowchart LR")
		expect(html).toContain("A--&gt;B")
	})

	it("renders KaTeX math output", () => {
		const html = renderMarkdown("Inline $x^2$ and $$\\int_0^1 x$$")
		expect(html).toContain('class="katex"')
	})

	it("escapes raw HTML instead of injecting it", () => {
		const html = renderMarkdown("<script>alert(1)</script>")
		expect(html).not.toContain("<script>alert")
		expect(html).toContain("&lt;script&gt;")
	})

	it("renders inline bold", () => {
		const html = renderMarkdown("El modelo **Llama-3.2-1B-Instruct-8bit** usa RAM")
		expect(html).toMatch(/<strong[^>]*>Llama-3\.2-1B-Instruct-8bit<\/strong>/)
		expect(html).not.toContain("**")
	})

	it("renders bullet lists with ul/li", () => {
		const html = renderMarkdown("- item one\n- item two")
		expect(html).toContain("<ul")
		expect(html).toContain("<li")
		expect(html).toContain("item one")
		expect(html).toContain("item two")
	})

	it("renders agent pseudo-list pattern as structured list with bold titles", () => {
		const html = renderMarkdown(
			"**Qwen2.5-1.5B-Instruct — la mejor relación\n**Qwen2.5-0.5B-Instruct — ~mitad de RAM",
		)
		expect(html).toContain("<ul")
		expect(html).toContain("<li")
		expect(html).toMatch(
			/<strong[^>]*>Qwen2\.5-1\.5B-Instruct<\/strong>/,
		)
		expect(html).not.toContain("**")
	})

	it("strips trailing incomplete bold when streaming", () => {
		const html = renderToStaticMarkup(
			<Markdown text="partial **bol" streaming />,
		)
		expect(html).not.toContain("**")
		expect(html).toContain("partial bol")
	})
})
