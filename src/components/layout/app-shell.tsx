import { useAtom, useSetAtom } from "jotai"
import { useState, type ReactNode } from "react"
import { ResizeHandle } from "@/components/layout/resize-handle"
import { useHorizontalResize } from "@/hooks/use-horizontal-resize"
import { cn } from "@/lib/utils"
import {
	DIFF_PANEL_WIDTH_MAX,
	DIFF_PANEL_WIDTH_MIN,
	diffPanelWidthAtom,
	setDiffPanelWidthAtom,
	setSidebarWidthAtom,
	SIDEBAR_WIDTH_MAX,
	SIDEBAR_WIDTH_MIN,
	sidebarWidthAtom,
} from "@/stores/atoms"

/** macOS overlay titlebar: keep this strip clear for traffic lights. */
const TRAFFIC_LIGHT_GUTTER = "w-[4.75rem] shrink-0"

interface AppShellProps {
	sidebar?: ReactNode
	children: ReactNode
	panel?: ReactNode
	panelOpen: boolean
	sidebarOpen: boolean
	className?: string
}

/** Minimal Palot-inspired desktop shell: sidebar + content. */
export function AppShell({
	sidebar,
	children,
	panel,
	panelOpen,
	sidebarOpen,
	className,
}: AppShellProps) {
	const [sidebarWidth] = useAtom(sidebarWidthAtom)
	const [diffPanelWidth] = useAtom(diffPanelWidthAtom)
	const setSidebarWidth = useSetAtom(setSidebarWidthAtom)
	const setDiffPanelWidth = useSetAtom(setDiffPanelWidthAtom)
	const [resizing, setResizing] = useState<"sidebar" | "diff" | null>(null)

	const sidebarResize = useHorizontalResize({
		width: sidebarWidth,
		onWidthChange: setSidebarWidth,
		min: SIDEBAR_WIDTH_MIN,
		max: SIDEBAR_WIDTH_MAX,
		onResizeStart: () => setResizing("sidebar"),
		onResizeEnd: () => setResizing(null),
	})

	const diffPanelResize = useHorizontalResize({
		width: diffPanelWidth,
		onWidthChange: setDiffPanelWidth,
		min: DIFF_PANEL_WIDTH_MIN,
		max: DIFF_PANEL_WIDTH_MAX,
		invertDelta: true,
		onResizeStart: () => setResizing("diff"),
		onResizeEnd: () => setResizing(null),
	})

	const shellTransition =
		"transition-[width] duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]"

	return (
		<div className={cn("flex h-full min-h-0 w-full overflow-hidden", className)}>
			{/*
			  Side chrome: transparent only (native vibrancy).
			  No CSS backdrop-filter here — stacking native blur + CSS blur flickers
			  at the edge with the solid chat card while dragging the window.
			*/}
			<aside
				className={cn(
					"flex h-full shrink-0 flex-col overflow-hidden bg-transparent",
					resizing !== "sidebar" && shellTransition,
				)}
				style={{ width: sidebarOpen ? sidebarWidth : 0 }}
			>
				<div
					className={cn(
						"flex h-full flex-col bg-transparent",
						!sidebarOpen && "pointer-events-none",
					)}
					style={{ width: sidebarWidth }}
					aria-hidden={!sidebarOpen}
				>
					{sidebar}
				</div>
			</aside>
			{sidebarOpen ? (
				<ResizeHandle onPointerDown={sidebarResize.onPointerDown} />
			) : null}
			{/* Solid chat surface + app bar — opaque layer, simple border (no soft shadow). */}
			<main className="solid-content relative z-10 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-[12px] border border-border">
				{children}
			</main>
			{panel && panelOpen ? (
				<ResizeHandle onPointerDown={diffPanelResize.onPointerDown} />
			) : null}
			{panel ? (
				<aside
					className={cn(
						"flex h-full shrink-0 flex-col overflow-hidden bg-transparent",
						resizing !== "diff" && shellTransition,
					)}
					style={{ width: panelOpen ? diffPanelWidth : 0 }}
				>
					<div
						className={cn(
							"flex h-full flex-col bg-transparent",
							!panelOpen && "pointer-events-none",
						)}
						style={{ width: diffPanelWidth }}
						aria-hidden={!panelOpen}
					>
						{panel}
					</div>
				</aside>
			) : null}
		</div>
	)
}

export { TRAFFIC_LIGHT_GUTTER }
