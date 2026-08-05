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
})
