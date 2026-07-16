import { collectSessionDiffs } from "@/lib/session-diffs"
import type { ChatMessage } from "@/types/acp"

export interface DiffLineStats {
	additions: number
	deletions: number
}

export function countLineDiffStats(
	oldText: string | undefined,
	newText: string,
): DiffLineStats {
	const oldLines = (oldText ?? "").split("\n")
	const newLines = newText.split("\n")
	let additions = 0
	let deletions = 0

	const max = Math.max(oldLines.length, newLines.length)
	for (let index = 0; index < max; index += 1) {
		const oldLine = oldLines[index]
		const newLine = newLines[index]

		if (oldLine === newLine) continue
		if (oldLine !== undefined) deletions += 1
		if (newLine !== undefined) additions += 1
	}

	return { additions, deletions }
}

export function collectSessionDiffStats(messages: ChatMessage[]): DiffLineStats {
	return collectSessionDiffs(messages).reduce<DiffLineStats>(
		(acc, entry) => {
			const stats = countLineDiffStats(entry.oldText, entry.newText)
			return {
				additions: acc.additions + stats.additions,
				deletions: acc.deletions + stats.deletions,
			}
		},
		{ additions: 0, deletions: 0 },
	)
}

export function hasDiffStats(stats: DiffLineStats): boolean {
	return stats.additions > 0 || stats.deletions > 0
}