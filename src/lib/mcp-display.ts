const MCP_TOKEN_LABELS: Record<string, string> = {
	craft: "Craft",
	figma: "Figma",
	framelink: "Framelink",
	minimax: "MiniMax",
	opencode: "OpenCode",
	paper: "Paper",
	playwright: "Playwright",
	supathings: "Supathings",
	xcode: "Xcode",
}

const MCP_ACRONYMS = new Set(["api", "http", "mcp", "sse", "stdio", "url"])

function splitMcpTokens(raw: string): string[] {
	const withCamelBoundaries = raw
		.replace(/([a-z])([A-Z])/g, "$1 $2")
		.replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")

	return withCamelBoundaries.split(/[-_\s/]+/).filter(Boolean)
}

function formatMcpToken(part: string): string {
	const key = part.trim().toLowerCase()
	if (MCP_TOKEN_LABELS[key]) return MCP_TOKEN_LABELS[key]
	if (MCP_ACRONYMS.has(key)) return key.toUpperCase()
	return part.charAt(0).toUpperCase() + part.slice(1).toLowerCase()
}

/** Human-readable label for MCP server ids (e.g. craft-business → Craft Business). */
export function formatMcpDisplayName(raw: string): string {
	const trimmed = raw.trim()
	if (!trimmed) return raw
	return splitMcpTokens(trimmed).map(formatMcpToken).join(" ")
}