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

	it("splits single-line GFM tables into one row per line", () => {
		const input =
			"| Modelo | Rol | Tamaño | |---|---|---| | **parakeet** | Transcripción | 461M | | **Llama** | Cleanup | 1.2G |"
		const result = normalizeAgentMarkdown(input)
		expect(result).toBe(
			"| Modelo | Rol | Tamaño |\n|---|---|---|\n| **parakeet** | Transcripción | 461M |\n| **Llama** | Cleanup | 1.2G |",
		)
	})

	it("splits inline GFM tables after intro text with blank line", () => {
		const input =
			"Claro, aquí tienes: | Anime | Estudio | |---|---| | SAO | A-1 | 2012 | x |"
		const result = normalizeAgentMarkdown(input)
		expect(result).toBe(
			"Claro, aquí tienes:\n\n| Anime | Estudio |\n|---|---|\n| SAO | A-1 | 2012 | x |",
		)
	})

	it("does not alter multi-line GFM tables", () => {
		const input = "| A | B |\n|---|---|\n| 1 | 2 |"
		expect(normalizeAgentMarkdown(input)).toBe(input)
	})

	it("unwraps fenced GFM tables", () => {
		const input =
			"Prueba:\n\n```markdown\n| Anime | Estudio |\n|-------|---------|\n| SAO | A-1 |\n```"
		const result = normalizeAgentMarkdown(input)
		expect(result).toContain("| Anime | Estudio |")
		expect(result).toContain("|-------|---------|")
		expect(result).not.toContain("```")
	})

	it("unwraps indented GFM tables", () => {
		const input =
			"Prueba:\n\n    | Anime | Estudio |\n    |-------|---------|\n    | SAO | A-1 |"
		const result = normalizeAgentMarkdown(input)
		expect(result).toContain("| Anime | Estudio |")
		expect(result).not.toMatch(/^\s{4}\|/m)
	})

	it("repairs separator column count to match header (agent shorthand)", () => {
		const input =
			"| Anime | Año | Protagonista | Mundo | |---|---| | Sword Art Online | 2012 | Kirito | VRMMO |"
		const result = normalizeAgentMarkdown(input)
		expect(result).toContain("| --- | --- | --- | --- |")
		expect(result).not.toContain("|---|---|")
	})

	it("inserts block break before inline headings", () => {
		const result = normalizeAgentMarkdown("Resumen: ## Título principal")
		expect(result).toBe("Resumen:\n\n## Título principal")
	})

	it("inserts block break before inline bullet lists", () => {
		const result = normalizeAgentMarkdown("Opciones: - uno\n- dos")
		expect(result).toBe("Opciones:\n\n- uno\n- dos")
	})

	it("splits inline numbered lists", () => {
		const result = normalizeAgentMarkdown("Pasos: 1. primero 2. segundo 3. tercero")
		expect(result).toBe("Pasos:\n\n1. primero\n 2. segundo\n 3. tercero")
	})

	it("inserts block break before inline code fences", () => {
		const result = normalizeAgentMarkdown("Mira esto: ```js\nconst x = 1\n```")
		expect(result).toBe("Mira esto:\n\n```js\nconst x = 1\n```")
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
