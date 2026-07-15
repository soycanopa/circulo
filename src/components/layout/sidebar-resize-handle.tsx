import { useCallback, useEffect, useState } from "react"
import { SHELL_INSET } from "@/lib/window-chrome"
import {
	SIDEBAR_MAX_WIDTH,
	SIDEBAR_MIN_WIDTH,
	setSidebarWidth,
} from "@/lib/preferences"

interface SidebarResizeHandleProps {
	width: number
	onWidthChange: (width: number) => void
}

export function SidebarResizeHandle({ width, onWidthChange }: SidebarResizeHandleProps) {
	const [dragging, setDragging] = useState(false)

	const onMouseMove = useCallback(
		(event: MouseEvent) => {
			const shell = document.querySelector('[data-slot="sidebar-wrapper"]')
			if (!shell) return
			const rect = shell.getBoundingClientRect()
			const next = Math.min(
				SIDEBAR_MAX_WIDTH,
				Math.max(SIDEBAR_MIN_WIDTH, event.clientX - rect.left - SHELL_INSET),
			)
			onWidthChange(next)
		},
		[onWidthChange],
	)

	useEffect(() => {
		if (!dragging) return

		const onMouseUp = () => {
			setDragging(false)
			setSidebarWidth(width)
		}

		window.addEventListener("mousemove", onMouseMove)
		window.addEventListener("mouseup", onMouseUp)
		return () => {
			window.removeEventListener("mousemove", onMouseMove)
			window.removeEventListener("mouseup", onMouseUp)
		}
	}, [dragging, onMouseMove, width])

	return (
		<div
			data-slot="sidebar-resize-handle"
			onMouseDown={() => setDragging(true)}
			className="relative z-20 shrink-0 cursor-col-resize select-none"
			style={{ width: SHELL_INSET }}
		>
			{dragging ? (
				<div
					className="absolute inset-y-0 left-1/2 w-1 -translate-x-1/2"
					style={{
						background:
							"linear-gradient(to bottom, rgba(0,0,0,0), rgba(111,203,243,0.45) 50%, rgba(0,0,0,0))",
					}}
				/>
			) : null}
		</div>
	)
}