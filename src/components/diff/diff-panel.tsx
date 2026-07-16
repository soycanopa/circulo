import { motion, useReducedMotion } from "motion/react"
import { DiffReviewPanel } from "@/components/diff/diff-review-panel"
import { RightPanelResizeHandle } from "@/components/layout/right-panel-resize-handle"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { terminalDrawer } from "@/lib/motion-presets"
import { SHELL_INSET } from "@/lib/window-chrome"

interface DiffPanelProps {
	width: number
	isResizing: boolean
	onWidthChange: (width: number) => void
	onResizingChange: (resizing: boolean) => void
}

export function DiffPanel({
	width,
	isResizing,
	onWidthChange,
	onResizingChange,
}: DiffPanelProps) {
	const reduceMotion = useReducedMotion()
	const motionTransition = isResizing || reduceMotion ? { duration: 0 } : terminalDrawer
	const panelWidth = width + SHELL_INSET

	return (
		<motion.aside
			data-slot="diff-panel"
			initial={reduceMotion ? false : { width: 0, opacity: 0 }}
			animate={{ width: panelWidth, opacity: 1 }}
			exit={{ width: 0, opacity: 0 }}
			transition={{
				width: motionTransition,
				opacity: motionTransition,
			}}
			className="flex h-full shrink-0 overflow-hidden"
			{...windowNoDragProps()}
		>
			<RightPanelResizeHandle
				width={width}
				onWidthChange={onWidthChange}
				onResizingChange={onResizingChange}
			/>
			<div
				data-slot="right-panel"
				className="h-full min-h-0 min-w-0 flex-1 overflow-hidden rounded-xl"
			>
				<DiffReviewPanel />
			</div>
		</motion.aside>
	)
}