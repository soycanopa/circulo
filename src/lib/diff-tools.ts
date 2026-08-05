import type { ChatMessage, ToolCall, ToolCallDiff } from "@/types/acp"

/** Files we auto-collapse in the review panel (lockfiles, build output, etc.). */
export const GENERATED_FILE_RE =
	/(^|[\\/])(package-lock\.json|pnpm-lock\.yaml|yarn\.lock|bun\.lockb?|dist|build|out|coverage|\.next|\.nuxt|node_modules|target)([\\/]|$)|\.(min\.|map$)/i

export interface SessionDiff {
	/** Aggregate diff for a single file across the whole session. */
	path: string
	oldText: string
	newText: string
	status: "created" | "modified" | "deleted" | "unchanged"
	generated: boolean
}

export function isGeneratedFile(path: string): boolean {
	return GENERATED_FILE_RE.test(path)
}

export function isDiffTool(tool: ToolCall): boolean {
	if (tool.kind === "diff") return true
	if (typeof tool.content === "object" && tool.content?.type === "diff") return true
	if (typeof tool.content === "string" && tool.content.startsWith("[diff ")) {
		return true
	}
	return tool.title.toLowerCase().includes("diff")
}

export function collectDiffTools(messages: ChatMessage[]): ToolCall[] {
	const seen = new Set<string>()
	const tools: ToolCall[] = []
	for (const message of messages) {
		for (const tool of message.toolCalls) {
			if (!isDiffTool(tool) || seen.has(tool.id)) continue
			seen.add(tool.id)
			tools.push(tool)
		}
	}
	return tools
}

function diffFromTool(tool: ToolCall): ToolCallDiff | null {
	if (typeof tool.content === "object" && tool.content?.type === "diff") {
		return tool.content
	}
	// String fallback keeps old/new indistinguishable — treat as modified blob.
	if (typeof tool.content === "string") {
		const pathMatch = /^\[diff (.+)\]/.exec(tool.content)
		const path = pathMatch?.[1]?.trim()
		if (!path) return null
		return { type: "diff", path, oldText: "", newText: tool.content }
	}
	return null
}

function diffStatus(tool: ToolCall): SessionDiff["status"] {
	const text = `${tool.title} ${typeof tool.content === "string" ? tool.content : ""}`
		.toLowerCase()
	if (/\bdelete|removed|remove\b/.test(text)) return "deleted"
	if (/\bcreate|created|new file|added\b/.test(text)) return "created"
	return "modified"
}

/**
 * Aggregate every diff tool call in the session into one entry per file path.
 * Later tool calls for the same path build on the earlier snapshot, so the
 * review panel shows the net change over the whole session.
 */
export function collectSessionDiffs(messages: ChatMessage[]): SessionDiff[] {
	const byPath = new Map<string, SessionDiff>()

	for (const message of messages) {
		for (const tool of message.toolCalls) {
			if (!isDiffTool(tool)) continue
			const diff = diffFromTool(tool)
			if (!diff) continue

			const existing = byPath.get(diff.path)
			if (existing) {
				// Net change over the session: keep the first snapshot as old,
				// advance new to the latest snapshot.
				if (!existing.oldText) existing.oldText = diff.oldText
				if (diff.newText) existing.newText = diff.newText
				if (existing.status === "unchanged") {
					existing.status = diffStatus(tool)
				} else if (diffStatus(tool) === "deleted") {
					existing.status = "deleted"
				}
			} else {
				byPath.set(diff.path, {
					path: diff.path,
					oldText: diff.oldText,
					newText: diff.newText,
					status: diffStatus(tool),
					generated: isGeneratedFile(diff.path),
				})
			}
		}
	}

	return [...byPath.values()].sort((a, b) => a.path.localeCompare(b.path))
}
