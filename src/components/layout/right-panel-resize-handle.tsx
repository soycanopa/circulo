import { useCallback, useEffect, useRef, useState } from "react"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { SHELL_INSET } from "@/lib/window-chrome"
import {
	RIGHT_PANEL_MAX_WIDTH,
	RIGHT_PANEL_MIN_WIDTH,
	setRightPanelWidth,
} from "@/lib/preferences"

interface RightPanelResizeHandleProps {
	width: number
	onWidthChange: (width: number) => void
	onResizingChange?: (resizing: boolean) => void
}

export function RightPanelResizeHandle({
	width,
	onWidthChange,
	onResizingChange,
}: RightPanelResizeHandleProps) {
	const [dragging, setDragging] = useState(false)
	const widthRef = useRef(width)
	widthRef.current = width

	const onMouseMove = useCallback(
		(event: MouseEvent) => {
			const shell = document.querySelector('[data-slot="sidebar-wrapper"]')
			if (!shell) return
			const rect = shell.getBoundingClientRect()
			const next = Math.min(
				RIGHT_PANEL_MAX_WIDTH,
				Math.max(RIGHT_PANEL_MIN_WIDTH, rect.right - event.clientX - SHELL_INSET),
			)
			onWidthChange(next)
		},
		[onWidthChange],
	)

	useEffect(() => {
		if (!dragging) return

		const onMouseUp = () => {
			setDragging(false)
			onResizingChange?.(false)
			setRightPanelWidth(widthRef.current)
		}

		window.addEventListener("mousemove", onMouseMove)
		window.addEventListener("mouseup", onMouseUp)
		return () => {
			window.removeEventListener("mousemove", onMouseMove)
			window.removeEventListener("mouseup", onMouseUp)
		}
	}, [dragging, onMouseMove, onResizingChange])

	return (
		<div
			data-slot="right-panel-resize-handle"
			{...windowNoDragProps()}
			onMouseDown={(event) => {
				event.preventDefault()
				onResizingChange?.(true)
				setDragging(true)
			}}
			className="relative z-[55] h-full shrink-0 cursor-col-resize select-none touch-none"
			style={{ width: SHELL_INSET }}
		>
			<div aria-hidden className="absolute inset-y-0 -left-2 -right-2" />
			{dragging ? (
				<div
					data-slot="sidebar-resize-indicator"
					className="pointer-events-none absolute inset-y-0 left-1/2 w-1 -translate-x-1/2"
				/>
			) : null}
		</div>
	)
}