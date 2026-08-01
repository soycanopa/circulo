/** Relative project paths referenced as `@path/to/file` in the composer. */
const MENTION_PATH_RE = /@([^\s@]+(?:\/[^\s@]+)*)/g

export function extractMentionPaths(text: string): string[] {
	const paths = new Set<string>()
	for (const match of text.matchAll(MENTION_PATH_RE)) {
		const path = match[1]?.trim()
		if (path) paths.add(path)
	}
	return [...paths]
}

/** Active `@query` at the cursor while the user is typing a mention. */
export function getActiveMention(
	text: string,
	cursor: number,
): { query: string; start: number } | null {
	const before = text.slice(0, cursor)
	const at = before.lastIndexOf("@")
	if (at === -1) return null
	if (at > 0 && !/\s/.test(before[at - 1] ?? "")) return null

	const query = before.slice(at + 1)
	if (/\s/.test(query)) return null

	return { query, start: at }
}

export function insertMention(
	text: string,
	start: number,
	cursor: number,
	path: string,
): { value: string; cursor: number } {
	const before = text.slice(0, start)
	const after = text.slice(cursor)
	const mention = `@${path}`
	const needsSpace = after.length > 0 && !/^\s/.test(after)
	const spacer = needsSpace ? " " : ""
	const value = `${before}${mention}${spacer}${after}`
	const nextCursor = before.length + mention.length + spacer.length
	return { value, cursor: nextCursor }
}
