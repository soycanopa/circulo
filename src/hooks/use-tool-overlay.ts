import { useAtom } from "jotai"
import { useCallback } from "react"
import { toolOverlayAtom } from "@/stores/atoms"
import type { ToolCallState } from "@/types/acp"

export function useToolOverlay() {
	const [overlay, setOverlay] = useAtom(toolOverlayAtom)

	const openTool = useCallback(
		(toolCall: ToolCallState) => {
			setOverlay({ type: "single", toolCall })
		},
		[setOverlay],
	)

	const openMultiDiff = useCallback(
		(toolCalls: ToolCallState[], activeId?: string) => {
			setOverlay({ type: "multi-diff", toolCalls, activeId })
		},
		[setOverlay],
	)

	const closeOverlay = useCallback(() => {
		setOverlay(null)
	}, [setOverlay])

	return {
		overlay,
		openTool,
		openMultiDiff,
		closeOverlay,
	}
}