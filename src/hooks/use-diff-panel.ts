import { useAtom, useSetAtom } from "jotai"
import { useCallback } from "react"
import { useToolOverlay } from "@/hooks/use-tool-overlay"
import { activeDiffToolIdAtom, diffPanelOpenAtom } from "@/stores/atoms"
import type { ToolCallState } from "@/types/acp"

export function useDiffPanel() {
	const [diffPanelOpen, setDiffPanelOpen] = useAtom(diffPanelOpenAtom)
	const setActiveDiffToolId = useSetAtom(activeDiffToolIdAtom)
	const { openTool, openMultiDiff } = useToolOverlay()

	const openDiff = useCallback(
		(toolCall: ToolCallState) => {
			if (!toolCall.diff) {
				openTool(toolCall)
				return
			}
			setActiveDiffToolId(toolCall.id)
			setDiffPanelOpen(true)
		},
		[openTool, setActiveDiffToolId, setDiffPanelOpen],
	)

	const openDiffs = useCallback(
		(toolCalls: ToolCallState[], activeId?: string) => {
			const withDiff = toolCalls.filter((tool) => tool.diff)
			if (withDiff.length === 0) return
			if (withDiff.length === 1) {
				openDiff(withDiff[0])
				return
			}
			setActiveDiffToolId(activeId ?? withDiff[withDiff.length - 1].id)
			setDiffPanelOpen(true)
		},
		[openDiff, setActiveDiffToolId, setDiffPanelOpen],
	)

	const openDiffFullscreen = useCallback(
		(toolCall: ToolCallState) => {
			openTool(toolCall)
		},
		[openTool],
	)

	const openDiffsFullscreen = useCallback(
		(toolCalls: ToolCallState[], activeId?: string) => {
			openMultiDiff(toolCalls, activeId)
		},
		[openMultiDiff],
	)

	const closeDiffPanel = useCallback(() => {
		setDiffPanelOpen(false)
	}, [setDiffPanelOpen])

	return {
		diffPanelOpen,
		setDiffPanelOpen,
		openDiff,
		openDiffs,
		openDiffFullscreen,
		openDiffsFullscreen,
		closeDiffPanel,
	}
}