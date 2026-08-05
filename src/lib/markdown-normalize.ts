/** Unicode asterisk variants → ASCII for markdown parsing. */
const UNICODE_ASTERISK = /[\uFF0A\u2217\u066D]/g

const EM_DASH_SEP = " — "

/** Line starts with `**` but has no closing `**` (agent pseudo-bullet). */
function isMalformedBoldLine(line: string): boolean {
	if (!line.startsWith("**")) return false
	return !line.slice(2).includes("**")
}

/** Convert `**title — description` → `- **title** — description`. */
function fixMalformedBoldLine(line: string): string {
	const inner = line.slice(2)
	const dashIdx = inner.indexOf(EM_DASH_SEP)
	if (dashIdx > 0) {
		const title = inner.slice(0, dashIdx)
		const desc = inner.slice(dashIdx)
		return `- **${title}**${desc}`
	}
	return `- **${inner}**`
}

/**
 * Strip trailing unclosed emphasis/code delimiters during streaming so partial
 * `**`, `__`, or `` ` `` don't flash as literal characters.
 */
export function stripIncompleteMarkdownDelimiters(text: string): string {
	let result = text

	const backtickCount = (result.match(/(?<!`)`(?!`)/g) ?? []).length
	if (backtickCount % 2 === 1) {
		const backtickMatch = /(`)([^`\n]*)$/.exec(result)
		if (backtickMatch) {
			result =
				result.slice(0, backtickMatch.index) + backtickMatch[2]
		}
	}

	const doubleStarCount = (result.match(/\*\*/g) ?? []).length
	if (doubleStarCount % 2 === 1) {
		const emphasisMatch = /(\*\*)([^*\n]*)$/.exec(result)
		if (emphasisMatch) {
			result =
				result.slice(0, emphasisMatch.index) + emphasisMatch[2]
		}
	}

	const doubleUnderscoreCount = (result.match(/__/g) ?? []).length
	if (doubleUnderscoreCount % 2 === 1) {
		const emphasisMatch = /(__)([^_\n]*)$/.exec(result)
		if (emphasisMatch) {
			result =
				result.slice(0, emphasisMatch.index) + emphasisMatch[2]
		}
	}

	return result
}

/**
 * Normalize common agent markdown mistakes before react-markdown parsing.
 * Skips fenced code blocks; fixes pseudo-list lines like `**Model — desc`.
 */
export function normalizeAgentMarkdown(text: string): string {
	const normalized = text.replace(UNICODE_ASTERISK, "*")

	const parts = normalized.split(/(```[\w]*\n[\s\S]*?```)/g)
	return parts
		.map((part, i) => {
			// Odd indices are fenced code blocks captured by the split
			if (i % 2 === 1) return part
			return part
				.split("\n")
				.map((line) =>
					isMalformedBoldLine(line) ? fixMalformedBoldLine(line) : line,
				)
				.join("\n")
		})
		.join("")
}

export function prepareMarkdownForRender(
	text: string,
	streaming = false,
): string {
	let prepared = text
	if (streaming) {
		prepared = stripIncompleteMarkdownDelimiters(prepared)
	}
	return normalizeAgentMarkdown(prepared)
}
