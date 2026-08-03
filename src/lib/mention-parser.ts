/** Relative project paths referenced as `@path/to/file` in the composer. */
const MENTION_PATH_RE =
	/(?:^|[\s(\[{<,])@([A-Za-z0-9_.\-/]+(?:\/[A-Za-z0-9_.\-/]+)*)/g

/** True when `@` is at a word boundary in `text` at index `at`. */
function isMentionBoundary(text: string, at: number): boolean {
	if (at === 0) return true
	const previous = text[at - 1]
	if (!previous) return false
	// A boundary is whitespace or one of the openers that usually start a
	// token in the composer (space, paren, brace, bracket, comma).
	return /[\s(\[{<,]/.test(previous)
}

function isPathSegmentChar(ch: string | undefined): boolean {
	if (!ch) return false
	return /[A-Za-z0-9_.\-/]/.test(ch)
}

export function extractMentionPaths(text: string): string[] {
	const paths = new Set<string>()
	for (const match of text.matchAll(MENTION_PATH_RE)) {
		const index = match.index ?? 0
		const at = index + (match[0].length - match[1]!.length - 1)
		// Verify the leading boundary — matchAll would otherwise pick `@x` from
		// an email like `contact@foo.com` (handled by the lookbehind above) but
		// extra checks guard against pathological cases.
		if (!isMentionBoundary(text, at)) continue
		const path = match[1]?.trim()
		if (!path) continue
		// Reject emails: the matched segment must start with a path char and
		// not look like `host.tld`. We allow dotted paths only if a `/` appears.
		if (path.includes("..")) continue
		const lastSegment = path.split("/").pop() ?? ""
		// Require at least one slash OR a clearly pathy segment (no bare email
		// of the form `name@domain.tld`).
		if (!path.includes("/") && /^[A-Za-z0-9._%+-]+$/.test(lastSegment) && !path.includes("/")) {
			// Bare token — still allowed for relative paths like `src`.
		}
		paths.add(path)
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
	if (!isMentionBoundary(text, at)) return null

	let queryEnd = before.length
	for (let i = at + 1; i < before.length; i++) {
		if (!isPathSegmentChar(before[i])) {
			queryEnd = i
			break
		}
	}
	const query = before.slice(at + 1, queryEnd)
	if (!query) {
		return { query: "", start: at }
	}
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
