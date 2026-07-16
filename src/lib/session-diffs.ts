import type { MultiDiffEntry } from "@/lib/tool-preview"
import { previewBadge } from "@/lib/tool-preview"
import type { ChatMessage } from "@/types/acp"

export interface SessionDiffEntry extends MultiDiffEntry {
	messageId: string
	timestamp: number
}

export function collectSessionDiffs(messages: ChatMessage[]): SessionDiffEntry[] {
	const latestByPath = new Map<string, SessionDiffEntry>()

	for (const message of messages) {
		if (message.role !== "assistant") continue
		for (const tool of message.toolCalls) {
			if (!tool.diff) continue
			latestByPath.set(tool.diff.path, {
				id: tool.id,
				messageId: message.id,
				timestamp: message.timestamp,
				title: tool.title,
				badge: previewBadge(tool),
				path: tool.diff.path,
				oldText: tool.diff.oldText,
				newText: tool.diff.newText,
			})
		}
	}

	return [...latestByPath.values()].sort((a, b) => a.path.localeCompare(b.path))
}

export function getLatestSessionDiff(messages: ChatMessage[]): SessionDiffEntry | null {
	let latest: SessionDiffEntry | null = null

	for (const message of messages) {
		if (message.role !== "assistant") continue
		for (const tool of message.toolCalls) {
			if (!tool.diff) continue
			const entry: SessionDiffEntry = {
				id: tool.id,
				messageId: message.id,
				timestamp: message.timestamp,
				title: tool.title,
				badge: previewBadge(tool),
				path: tool.diff.path,
				oldText: tool.diff.oldText,
				newText: tool.diff.newText,
			}
			if (!latest || entry.timestamp >= latest.timestamp) {
				latest = entry
			}
		}
	}

	return latest
}