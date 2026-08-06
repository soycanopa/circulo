/** Unicode asterisk variants → ASCII for markdown parsing. */
const UNICODE_ASTERISK = /[\uFF0A\u2217\u066D]/g

const EM_DASH_SEP = " — "

/** GFM separator row marker, e.g. `|---|---|` or `|:---:|`. */
const GFM_TABLE_SEPARATOR = /\|[-: ]+\|/

/** Detects the start of a GFM table (`| col | ... | |---`). */
const GFM_TABLE_START = /\|(?:[^|\n]+\|)+\s*\|[-:]/

/** Opening fence: optional language tag, optional whitespace, optional newline. */
const FENCED_BLOCK =
	/```(?:[a-zA-Z0-9_-]+)?[^\S\n]*\n?([\s\S]*?)```/g

/** Indented block of GFM table rows (4+ spaces or tab). */
const INDENTED_TABLE_BLOCK =
	/(?:^|\n)((?:[ \t]{4,}\|[^\n]*\n?)+)/g

function isGfmTableLine(line: string): boolean {
	const trimmed = line.trim()
	if (!trimmed.startsWith("|")) return false
	return GFM_TABLE_SEPARATOR.test(trimmed) || /\|[^|\n]+\|/.test(trimmed)
}

function isGfmTableContent(text: string): boolean {
	const lines = text
		.split("\n")
		.map((line) => line.trim())
		.filter((line) => line.length > 0)
	if (lines.length < 2) return false
	if (!lines.some((line) => GFM_TABLE_SEPARATOR.test(line))) return false
	const tableLines = lines.filter(isGfmTableLine)
	if (tableLines.length < 2) return false
	const nonTable = lines.length - tableLines.length
	return nonTable <= 2
}

function parseTableRow(line: string): string[] {
	const trimmed = line.trim()
	if (!trimmed.startsWith("|")) return []
	const inner = trimmed.replace(/^\|/, "").replace(/\|$/, "")
	if (!inner) return []
	return inner.split("|").map((cell) => cell.trim())
}

function isSeparatorRow(line: string): boolean {
	const cells = parseTableRow(line)
	return (
		cells.length > 0 &&
		cells.every((cell) => /^:?-{3,}:?$/.test(cell))
	)
}

function makeSeparatorRow(columnCount: number): string {
	if (columnCount <= 0) return ""
	return `| ${Array(columnCount).fill("---").join(" | ")} |`
}

/** Agents often emit `|---|---|` for a 4-column header — GFM needs matching widths. */
function repairGfmTableSeparators(text: string): string {
	const lines = text.split("\n")
	const out: string[] = []
	let i = 0

	while (i < lines.length) {
		const line = lines[i]
		const trimmed = line.trim()
		const nextLine = lines[i + 1]
		const nextTrimmed = nextLine?.trim() ?? ""

		if (
			trimmed.startsWith("|") &&
			!isSeparatorRow(trimmed) &&
			nextTrimmed &&
			isSeparatorRow(nextTrimmed)
		) {
			const colCount = parseTableRow(trimmed).length
			const sepColCount = parseTableRow(nextTrimmed).length
			out.push(line)
			i++
			if (colCount > 0 && colCount !== sepColCount) {
				const indent = nextLine?.match(/^(\s*)/)?.[1] ?? ""
				out.push(indent + makeSeparatorRow(colCount))
			} else {
				out.push(nextLine)
			}
			i++
			continue
		}

		out.push(line)
		i++
	}

	return out.join("\n")
}

/** Split collapsed GFM table rows onto separate lines. */
function collapseTableRows(table: string): string {
	if (!GFM_TABLE_SEPARATOR.test(table)) return table

	let result = table
	// Header row immediately before the separator row (`| ... | |---`).
	result = result.replace(/ \| \|---/g, " |\n|---")
	// Separator row immediately before the first data row (`|---| | ...`).
	result = result.replace(/(\|[-:]+(?:\|[-:]+)*\|) \| /g, "$1\n| ")
	// Data row before the next data row (`... | | ...`).
	result = result.replace(/ \| \|(?!-)/g, " |\n|")
	return repairGfmTableSeparators(result)
}

/** Normalize a GFM table embedded anywhere on a line. */
function normalizeGfmTablesInLine(line: string): string {
	const match = GFM_TABLE_START.exec(line)
	if (!match || match.index === undefined) return line

	const prefix = line.slice(0, match.index).trimEnd()
	const tablePart = collapseTableRows(line.slice(match.index))

	if (!prefix) return tablePart
	return `${prefix}\n\n${tablePart}`
}

function normalizeGfmTablesInText(text: string): string {
	return repairGfmTableSeparators(
		text.split("\n").map(normalizeGfmTablesInLine).join("\n"),
	)
}

/** Turn fenced or indented table blocks into plain GFM tables. */
function unwrapFencedGfmTables(text: string): string {
	return text.replace(FENCED_BLOCK, (full, inner: string) => {
		const body = inner.replace(/\r\n/g, "\n").trim()
		if (!isGfmTableContent(body)) return full

		const lines = body
			.split("\n")
			.map((line) => line.trim())
			.filter((line) => line.length > 0)
		const tableLines = lines.filter(isGfmTableLine)
		const introLines = lines.filter((line) => !isGfmTableLine(line))
		const tableBlock = collapseTableRows(tableLines.join("\n"))

		if (introLines.length === 0) return `\n\n${tableBlock}\n\n`
		return `\n\n${introLines.join("\n")}\n\n${tableBlock}\n\n`
	})
}

function normalizeIndentedGfmTables(text: string): string {
	return text.replace(INDENTED_TABLE_BLOCK, (match, block: string) => {
		const lines = block
			.trimEnd()
			.split("\n")
			.map((line: string) => line.replace(/^[ \t]{4,}/, "").trim())
			.filter((line: string) => line.length > 0)
		if (!isGfmTableContent(lines.join("\n"))) return match

		const tableBlock = collapseTableRows(lines.join("\n"))
		return `${match.startsWith("\n") ? "\n\n" : ""}${tableBlock}\n\n`
	})
}

/** `Resumen: ## Título` → `Resumen:\n\n## Título` */
function normalizeInlineHeadings(text: string): string {
	return text.replace(/([.:!?])(\s*)(#{1,6}\s)/g, "$1\n\n$3")
}

/** `Opciones: - uno` → `Opciones:\n\n- uno` */
function normalizeInlineBulletLists(text: string): string {
	return text.replace(/([.:!?])(\s*)- /g, "$1\n\n- ")
}

/** `Pasos: 1. a 2. b` → multiline ordered list. */
function normalizeInlineNumberedLists(text: string): string {
	let result = text.replace(/([.:!?])(\s*)(\d+\.\s)/g, "$1\n\n$3")
	result = result.replace(/(\d+\.\s[^\n]+?)(?=\s+\d+\.\s)/g, "$1\n")
	return result
}

/** `Mira esto: ```js` → `Mira esto:\n\n```js` */
function normalizeInlineCodeFences(text: string): string {
	return text.replace(/([.:!?])(\s*)(```)/g, "$1\n\n$3")
}

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

function normalizeMalformedBoldLines(text: string): string {
	return text
		.split("\n")
		.map((line) =>
			isMalformedBoldLine(line) ? fixMalformedBoldLine(line) : line,
		)
		.join("\n")
}

function normalizeProseSegment(text: string): string {
	let result = text
	result = normalizeIndentedGfmTables(result)
	result = normalizeGfmTablesInText(result)
	result = normalizeInlineHeadings(result)
	result = normalizeInlineBulletLists(result)
	result = normalizeInlineNumberedLists(result)
	result = normalizeMalformedBoldLines(result)
	return result
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
	// Unwrap table-only code fences before fenced-block splitting.
	const withTablesUnwrapped = unwrapFencedGfmTables(normalized)
	// Run before fenced-block split so `text: ```lang` gets a block break.
	const withCodeBreaks = normalizeInlineCodeFences(withTablesUnwrapped)

	const parts = withCodeBreaks.split(/(```[\w]*\n?[\s\S]*?```)/g)
	return parts
		.map((part, i) => {
			// Odd indices are fenced code blocks captured by the split
			if (i % 2 === 1) return part
			return normalizeProseSegment(part)
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
