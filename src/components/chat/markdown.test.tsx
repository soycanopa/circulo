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

	it("renders agent single-line GFM tables", () => {
		const html = renderMarkdown(
			"| Modelo | Rol | Tamaño | |---|---|---| | **parakeet-tdt-0.6b-v3** | Transcripción | 461M | | **Llama-3.2-1B-Instruct-8bit** | Cleanup | 1.2G |",
		)
		expect(html).toContain("<table")
		expect(html).toMatch(
			/<strong[^>]*>parakeet-tdt-0\.6b-v3<\/strong>/,
		)
		expect(html).toMatch(
			/<strong[^>]*>Llama-3\.2-1B-Instruct-8bit<\/strong>/,
		)
		expect(html).not.toContain("|---|---|")
	})

	it("renders inline GFM tables after intro text (anime case)", () => {
		const html = renderMarkdown(
			"Claro, aquí tienes una tabla con algunos isekai populares: | Anime | Estudio | Año | Nota | |---|---|---|---| | Sword Art Online | A-1 Pictures | 2012 | Precursor del género moderno | | Re:Zero | White Fox | 2016 | Drama psicológico |",
		)
		expect(html).toContain("<table")
		expect(html).toContain("Sword Art Online")
		expect(html).toContain("Re:Zero")
		expect(html).not.toContain("|---|---|")
	})

	it("renders fenced GFM tables as HTML tables", () => {
		const html = renderMarkdown(
			"Prueba con esta versión:\n\n```markdown\n| Anime | Estudio | Año | Nota |\n|-------|---------|-----|------|\n| Sword Art Online | A-1 Pictures | 2012 | Precursor del género |\n| Re:Zero | White Fox | 2016 | Drama psicológico |\n```",
		)
		expect(html).toContain("<table")
		expect(html).toContain("Sword Art Online")
		expect(html).toContain("Re:Zero")
		expect(html).not.toContain("<pre")
	})

	it("renders indented GFM tables as HTML tables", () => {
		const html = renderMarkdown(
			"Prueba:\n\n    | Anime | Estudio |\n    |-------|---------|\n    | SAO | A-1 |\n    | Re:Zero | White Fox |",
		)
		expect(html).toContain("<table")
		expect(html).toContain("SAO")
		expect(html).toContain("Re:Zero")
		expect(html).not.toContain("<pre")
	})

	it("renders isekai table with short separator row", () => {
		const html = renderMarkdown(
			"Aquí tienes una tabla de los isekai más populares:\n\n| Anime | Año | Protagonista | Mundo | |---|---| | Sword Art Online | 2012 | Kirito | VRMMO atrapado | | No Game No Life | 2014 | Sora & Shiro | Mundo regido por juegos | | Re:Zero | 2016 | Subaru | Reino de Lugunica (magia) |",
		)
		expect(html).toContain("<table")
		expect(html).toContain("Sword Art Online")
		expect(html).toContain("Re:Zero")
		expect(html).not.toContain("|---|---|")
	})

	it("renders inline headings after punctuation", () => {
		const html = renderMarkdown("Resumen: ## Título principal")
		expect(html).toContain("<h2")
		expect(html).toContain("Título principal")
	})

	it("renders inline bullet lists after punctuation", () => {
		const html = renderMarkdown("Opciones: - uno\n- dos")
		expect(html).toContain("<ul")
		expect(html).toContain("uno")
		expect(html).toContain("dos")
	})

	it("renders inline numbered lists", () => {
		const html = renderMarkdown("Pasos: 1. primero 2. segundo 3. tercero")
		expect(html).toContain("<ol")
		expect(html).toContain("primero")
		expect(html).toContain("tercero")
	})

	it("renders inline code fences after punctuation", () => {
		const html = renderMarkdown("Mira esto: ```js\nconst x = 1\n```")
		expect(html).toContain("<pre")
		expect(html).toContain("hljs")
		expect(html).not.toMatch(/Mira esto: ```/)
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
