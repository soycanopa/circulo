import {
	createContext,
	useCallback,
	useContext,
	useEffect,
	useRef,
	useState,
	type CSSProperties,
	type ReactNode,
} from "react"
import type { LucideIcon } from "lucide-react"

import { AnimatePresence, motion, useReducedMotion } from "motion/react"
import { DiffReviewPanel } from "@/components/diff/diff-review-panel"
import { AppBar } from "@/components/layout/app-bar"
import { DiffToggleButton } from "@/components/layout/diff-toggle-button"
import { RightPanelResizeHandle } from "@/components/layout/right-panel-resize-handle"
import { SidebarResizeHandle } from "@/components/layout/sidebar-resize-handle"
import { TerminalToggleButton } from "@/components/layout/terminal-toggle-button"
import { WindowControls } from "@/components/layout/window-controls"
import { WindowDragStrip } from "@/components/layout/window-drag-strip"
import { useDiffPanelAutoOpen } from "@/hooks/use-diff-panel-auto-open"
import { windowDragRegionProps, windowNoDragProps } from "@/hooks/use-window-drag"
import { useSessions } from "@/hooks/use-sessions"

import { panelEase } from "@/lib/motion-presets"
import { getRightPanelWidth, getSidebarWidth } from "@/lib/preferences"
import { diffPanelOpenAtom, terminalOpenAtom } from "@/stores/atoms"
import { useAtomValue, useSetAtom } from "jotai"
import {
	APP_BAR_CONTROL_PADDING_TOP,
	APP_BAR_HEIGHT,
	APP_BAR_TITLE_GAP,
	APP_BAR_TITLE_INSET_LEFT,
	APP_BAR_TITLE_INSET_LEFT_COLLAPSED,
	APP_BAR_TITLE_PADDING_TOP,
	SHELL_INSET,
	SIDEBAR_COLLAPSE_THRESHOLD,
	WINDOW_CONTROLS_END,
} from "@/lib/window-chrome"
import { cn } from "@/lib/utils"

interface SidebarLayoutProps {
	sidebar: ReactNode
	children: ReactNode
	appBar?: ReactNode
}

interface SidebarContextValue {
	open: boolean
	setOpen: (open: boolean) => void
	toggleSidebar: () => void
}

const SidebarContext = createContext<SidebarContextValue | null>(null)

export function useSidebarLayout() {
	const context = useContext(SidebarContext)
	if (!context) {
		throw new Error("useSidebarLayout must be used within SidebarLayout")
	}
	return context
}

function NarrowWindowCollapser({ open, setOpen }: { open: boolean; setOpen: (open: boolean) => void }) {
	const collapsedByUsRef = useRef(false)

	useEffect(() => {
		const check = () => {
			const narrow = window.innerWidth < SIDEBAR_COLLAPSE_THRESHOLD
			if (narrow && open) {
				collapsedByUsRef.current = true
				setOpen(false)
			} else if (!narrow && !open && collapsedByUsRef.current) {
				collapsedByUsRef.current = false
				setOpen(true)
			} else if (!narrow && !open) {
				collapsedByUsRef.current = false
			}
		}

		check()
		window.addEventListener("resize", check)
		return () => window.removeEventListener("resize", check)
	}, [open, setOpen])

	return null
}

function LayoutWindowControls({
	open,
	onToggleSidebar,
}: {
	open: boolean
	onToggleSidebar: () => void
}) {
	const { newThread } = useSessions()
	return (
		<WindowControls
			sidebarOpen={open}
			onToggleSidebar={onToggleSidebar}
			onNewThread={() => void newThread()}
		/>
	)
}

function ChromeShortcutListener() {
	const setTerminalOpen = useSetAtom(terminalOpenAtom)
	const setDiffPanelOpen = useSetAtom(diffPanelOpenAtom)

	useEffect(() => {
		const handleKeyDown = (event: KeyboardEvent) => {
			if (!(event.metaKey || event.ctrlKey)) return
			if (event.key === "j" && !event.shiftKey) {
				event.preventDefault()
				setTerminalOpen((open) => !open)
				return
			}
			if (event.key === "d" && event.shiftKey) {
				event.preventDefault()
				setDiffPanelOpen((open) => !open)
			}
		}

		window.addEventListener("keydown", handleKeyDown)
		return () => window.removeEventListener("keydown", handleKeyDown)
	}, [setTerminalOpen, setDiffPanelOpen])

	return null
}

function DiffPanelAutoOpen() {
	useDiffPanelAutoOpen()
	return null
}

export function SidebarLayout({ sidebar, children, appBar }: SidebarLayoutProps) {
	const [open, setOpen] = useState(true)
	const [sidebarWidth, setSidebarWidth] = useState(getSidebarWidth)
	const [rightPanelWidth, setRightPanelWidth] = useState(getRightPanelWidth)
	const [isResizing, setIsResizing] = useState(false)
	const [isRightResizing, setIsRightResizing] = useState(false)
	const diffPanelOpen = useAtomValue(diffPanelOpenAtom)
	const reduceMotion = useReducedMotion()

	const toggleSidebar = useCallback(() => {
		setOpen((value) => !value)
	}, [])

	const titleLeft = open
		? sidebarWidth + SHELL_INSET + 16 + APP_BAR_TITLE_INSET_LEFT
		: WINDOW_CONTROLS_END + APP_BAR_TITLE_GAP + APP_BAR_TITLE_INSET_LEFT_COLLAPSED

	useEffect(() => {
		const handleKeyDown = (event: KeyboardEvent) => {
			if (event.key === "b" && (event.metaKey || event.ctrlKey)) {
				event.preventDefault()
				toggleSidebar()
			}
		}

		window.addEventListener("keydown", handleKeyDown)
		return () => window.removeEventListener("keydown", handleKeyDown)
	}, [toggleSidebar])

	return (
		<SidebarContext.Provider value={{ open, setOpen, toggleSidebar }}>
			<div
				data-slot="sidebar-wrapper"
				data-state={open ? "expanded" : "collapsed"}
				className="group/sidebar-wrapper relative flex h-screen w-full gap-0 p-2 text-foreground"
				style={
					{
						"--shell-inset": `${SHELL_INSET}px`,
						"--sidebar-width": `${sidebarWidth}px`,
						"--window-controls-end": `${WINDOW_CONTROLS_END}px`,
						"--app-bar-title-gap": `${APP_BAR_TITLE_GAP}px`,
					} as CSSProperties
				}
			>
				<NarrowWindowCollapser open={open} setOpen={setOpen} />
				<ChromeShortcutListener />
				<DiffPanelAutoOpen />

				<aside
					data-slot="sidebar"
					data-state={open ? "expanded" : "collapsed"}
					data-resizing={isResizing ? "" : undefined}
					className="relative flex h-full shrink-0 flex-col overflow-hidden rounded-xl"
					style={{
						width: open ? sidebarWidth : 0,
						pointerEvents: open ? "auto" : "none",
					}}
				>
					<div
						className="flex h-full min-w-0 flex-col"
						style={{ width: sidebarWidth }}
					>
						<SidebarChromeHeader />
						{sidebar}
					</div>
				</aside>

				{open ? (
					<div
						className="relative z-[55] shrink-0 self-stretch overflow-visible"
						style={{ width: SHELL_INSET }}
					>
						<SidebarResizeHandle
							width={sidebarWidth}
							onWidthChange={setSidebarWidth}
							onResizingChange={setIsResizing}
						/>
					</div>
				) : null}

				<main
					data-slot="sidebar-inset"
					className="flex min-w-0 flex-1 flex-col overflow-hidden rounded-xl"
				>
					<AppBar />
					<div className="flex min-h-0 min-w-0 flex-1 overflow-hidden">
						<div
							data-slot="content-area"
							className="relative min-h-0 min-w-0 flex-1 overflow-hidden"
						>
							{children}
						</div>
						<AnimatePresence initial={false}>
							{diffPanelOpen ? (
								<motion.div
									key="diff-panel-shell"
									initial={reduceMotion ? false : { width: 0, opacity: 0 }}
									animate={{ width: rightPanelWidth + SHELL_INSET, opacity: 1 }}
									exit={{ width: 0, opacity: 0 }}
									transition={
										isRightResizing || reduceMotion ? { duration: 0 } : panelEase
									}
									className="flex h-full shrink-0 overflow-hidden"
								>
									<RightPanelResizeHandle
										width={rightPanelWidth}
										onWidthChange={setRightPanelWidth}
										onResizingChange={setIsRightResizing}
									/>
									<div
										data-slot="right-panel"
										className="h-full min-h-0 min-w-0 flex-1 overflow-hidden"
									>
										<DiffReviewPanel />
									</div>
								</motion.div>
							) : null}
						</AnimatePresence>
					</div>
				</main>

				<WindowDragStrip />
				{appBar ? (
					<div
						data-slot="session-title-layer"
						data-resizing={isResizing ? "" : undefined}
						className="pointer-events-none absolute z-[52] box-border flex items-start overflow-visible"
						style={{
							left: titleLeft,
							top: 0,
							height: APP_BAR_HEIGHT,
							right: SHELL_INSET + 12,
							paddingTop: APP_BAR_TITLE_PADDING_TOP,
						}}
					>
						<div
							{...windowNoDragProps()}
							className="pointer-events-auto flex min-w-0 flex-1 items-center overflow-visible"
						>
							{appBar}
						</div>
					</div>
				) : null}
				<div
					data-slot="app-bar-actions-layer"
					className="pointer-events-none absolute z-[52] flex items-start justify-end gap-0.5"
					style={{
						top: 0,
						right: SHELL_INSET + 12,
						height: APP_BAR_HEIGHT,
						paddingTop: APP_BAR_CONTROL_PADDING_TOP,
					}}
				>
					<DiffToggleButton />
					<TerminalToggleButton />
				</div>
				<LayoutWindowControls open={open} onToggleSidebar={toggleSidebar} />
			</div>
		</SidebarContext.Provider>
	)
}

function SidebarChromeHeader() {
	return (
		<div
			data-slot="sidebar-chrome-header"
			{...windowDragRegionProps()}
			className="relative z-[45] box-border shrink-0 border-b border-border/50"
			style={{ height: APP_BAR_HEIGHT }}
		/>
	)
}

export function Sidebar({ className, children }: { className?: string; children: ReactNode }) {
	return (
		<div
			data-slot="sidebar-inner"
			className={cn("flex min-h-0 w-full min-w-0 flex-1 flex-col", className)}
		>
			{children}
		</div>
	)
}

export function SidebarHeader({ children }: { children: ReactNode }) {
	return (
		<div className="flex shrink-0 items-center gap-2 border-b border-sidebar-border/10 px-4 py-2">
			{children}
		</div>
	)
}

export function SidebarContent({ children }: { children: ReactNode }) {
	return <div className="scrollbar-thin flex-1 overflow-y-auto px-2 py-2">{children}</div>
}

export function SidebarFooter({ children }: { children: ReactNode }) {
	return (
		<div data-slot="sidebar-footer" className="w-full shrink-0 space-y-1 p-2">
			{children}
		</div>
	)
}

export function SidebarGroup({
	children,
	label,
	icon: Icon,
}: {
	children: ReactNode
	label?: string
	icon?: LucideIcon
}) {
	return (
		<div className="relative py-1">
			{label ? (
				<p className="flex items-center gap-1.5 px-2 py-1.5 text-xs font-medium text-muted-foreground">
					{Icon ? <Icon className="size-3 shrink-0 opacity-80" /> : null}
					{label}
				</p>
			) : null}
			{children}
		</div>
	)
}

export function SidebarMenu({
	children,
	className,
}: {
	children: ReactNode
	className?: string
}) {
	return <ul className={cn("flex flex-col gap-0.5", className)}>{children}</ul>
}

export function SidebarMenuItem({ children }: { children: ReactNode }) {
	return <li>{children}</li>
}

interface SidebarMenuButtonProps {
	children: ReactNode
	isActive?: boolean
	onClick?: () => void
	className?: string
	size?: "default" | "sm"
	disabled?: boolean
}

export function SidebarMenuButton({
	children,
	isActive,
	onClick,
	className,
	size = "default",
	disabled,
}: SidebarMenuButtonProps) {
	return (
		<button
			type="button"
			data-slot="sidebar-menu-button"
			data-active={isActive ? "" : undefined}
			disabled={disabled}
			onClick={onClick}
			className={cn(
				"flex w-full items-center gap-2 rounded-md px-2 text-left text-sidebar-foreground transition-colors",
				size === "default" ? "h-8 text-[13px]" : "h-7 text-xs",
				isActive && "text-sidebar-accent-foreground",
				className,
			)}
		>
			{children}
		</button>
	)
}