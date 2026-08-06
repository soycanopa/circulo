import { useAtom, useSetAtom } from "jotai"
import { useLayoutEffect, useRef, useState, type ReactNode } from "react"
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

/** Shared shell slide easing — decelerates into place like native macOS drawers. */
const SHELL_TRANSITION =
	"transition-[width] duration-[380ms] ease-[cubic-bezier(0.32,0.72,0,1)]"
const PANEL_CONTENT_TRANSITION =
	"transition-[opacity,transform] duration-[320ms] ease-[cubic-bezier(0.32,0.72,0,1)]"

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

	// Keep panel mounted through close animation; panel prop becomes null on close.
	const lastPanelRef = useRef(panel)
	if (panel) lastPanelRef.current = panel
	const panelContent = panel ?? lastPanelRef.current

	const [panelInDom, setPanelInDom] = useState(panelOpen)
	const [panelShown, setPanelShown] = useState(false)
	const panelMounted = panelInDom || panelOpen

	useLayoutEffect(() => {
		if (panelOpen) {
			setPanelInDom(true)
			setPanelShown(false)
			const frame = requestAnimationFrame(() => {
				requestAnimationFrame(() => setPanelShown(true))
			})
			return () => cancelAnimationFrame(frame)
		}
		setPanelShown(false)
	}, [panelOpen])

	const handlePanelTransitionEnd = (
		event: React.TransitionEvent<HTMLElement>,
	) => {
		if (event.propertyName !== "width") return
		if (!panelOpen && !panelShown) setPanelInDom(false)
	}

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

	return (
		<div className={cn("flex h-full min-h-0 w-full overflow-hidden", className)}>
			{/*
			  Side chrome: inherits fixed gray frost from #root (no desktop tint).
			  No CSS backdrop-filter — avoids edge flicker with the solid chat card.
			*/}
			<aside
				className={cn(
					"chrome-frost flex h-full shrink-0 flex-col overflow-hidden",
					resizing !== "sidebar" && SHELL_TRANSITION,
				)}
				style={{ width: sidebarOpen ? sidebarWidth : 0 }}
			>
				<div
					className={cn(
						"flex h-full flex-col",
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
			<main className="solid-content relative z-10 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-[12px]">
				{children}
			</main>
			{panelMounted && panelContent ? (
				<ResizeHandle onPointerDown={diffPanelResize.onPointerDown} />
			) : null}
			{panelMounted && panelContent ? (
				<aside
					className={cn(
						"chrome-frost flex h-full shrink-0 flex-col overflow-hidden",
						resizing !== "diff" && SHELL_TRANSITION,
					)}
					style={{ width: panelShown ? diffPanelWidth : 0 }}
					onTransitionEnd={handlePanelTransitionEnd}
				>
					<div
						className={cn(
							"flex h-full flex-col",
							PANEL_CONTENT_TRANSITION,
							panelShown
								? "translate-x-0 opacity-100"
								: "pointer-events-none translate-x-3 opacity-0",
						)}
						style={{ width: diffPanelWidth }}
						aria-hidden={!panelShown}
					>
						{panelContent}
					</div>
				</aside>
			) : null}
		</div>
	)
}

export { TRAFFIC_LIGHT_GUTTER }
