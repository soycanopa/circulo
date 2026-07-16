import { codeToHtml } from "shiki"

export const SHIKI_THEME = "github-dark-default"

const CACHE_MAX_ENTRIES = 200

const LANGUAGE_ALIASES: Record<string, string> = {
	bash: "bash",
	sh: "bash",
	shell: "bash",
	zsh: "bash",
	js: "javascript",
	ts: "typescript",
	tsx: "tsx",
	jsx: "jsx",
	py: "python",
	rb: "ruby",
	rs: "rust",
	go: "go",
	golang: "go",
	yml: "yaml",
	md: "markdown",
	jsonc: "json",
	text: "text",
	plaintext: "text",
}

const highlightCache = new Map<string, string>()

export function normalizeHighlightLanguage(language: string | undefined): string {
	const raw = (language ?? "text").trim().toLowerCase()
	return LANGUAGE_ALIASES[raw] ?? raw
}

function cacheKey(language: string, code: string, theme: string): string {
	return `${theme}\0${language}\0${code}`
}

export async function highlightCode(
	code: string,
	language: string | undefined,
	theme: string = SHIKI_THEME,
): Promise<string> {
	const lang = normalizeHighlightLanguage(language)
	const key = cacheKey(lang, code, theme)
	const cached = highlightCache.get(key)
	if (cached) return cached

	try {
		const html = await codeToHtml(code, { lang, theme })
		if (highlightCache.size >= CACHE_MAX_ENTRIES) {
			const oldest = highlightCache.keys().next().value
			if (oldest) highlightCache.delete(oldest)
		}
		highlightCache.set(key, html)
		return html
	} catch {
		const escaped = code
			.replace(/&/g, "&amp;")
			.replace(/</g, "&lt;")
			.replace(/>/g, "&gt;")
		return `<pre class="shiki-fallback"><code>${escaped}</code></pre>`
	}
}