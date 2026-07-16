import { useAtom, useAtomValue } from "jotai"
import { Plus, TerminalSquare, X } from "lucide-react"
import { motion, useReducedMotion } from "motion/react"
import { useCallback, useEffect, useRef, useState } from "react"
import { TerminalTabPane } from "@/components/terminal/terminal-tab-pane"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { terminalDrawer } from "@/lib/motion-presets"
import { clampTerminalHeight, TERMINAL_SURFACE } from "@/lib/terminal"
import { isTauri } from "@/lib/window-chrome"
import { cn } from "@/lib/utils"
import { projectPathAtom, terminalHeightAtom, terminalOpenAtom } from "@/stores/atoms"
import "@xterm/xterm/css/xterm.css"

interface TerminalTab {
	id: string
	title: string
	cwd: string | null
}

interface TerminalTabApi {
	fit: () => void
	focus: () => void
}

function createTab(index: number, cwd: string | null): TerminalTab {
	return {
		id: crypto.randomUUID(),
		title: `Terminal ${index}`,
		cwd,
	}
}

export function TerminalPanel() {
	const projectPath = useAtomValue(projectPathAtom)
	const [height, setHeight] = useAtom(terminalHeightAtom)
	const [, setTerminalOpen] = useAtom(terminalOpenAtom)
	const [tabs, setTabs] = useState<TerminalTab[]>(() => [createTab(1, projectPath)])
	const [activeTabId, setActiveTabId] = useState(() => tabs[0]?.id ?? "")
	const tabApisRef = useRef(new Map<string, TerminalTabApi>())
	const resizeStateRef = useRef<{ startY: number; startHeight: number } | null>(null)
	const [isResizing, setIsResizing] = useState(false)
	const reduceMotion = useReducedMotion()

	useEffect(() => {
		if (!activeTabId && tabs[0]) setActiveTabId(tabs[0].id)
	}, [activeTabId, tabs])

	const registerTabApi = useCallback((tabId: string, api: TerminalTabApi) => {
		tabApisRef.current.set(tabId, api)
	}, [])

	const fitActiveTab = useCallback(() => {
		const api = tabApisRef.current.get(activeTabId)
		api?.fit()
	}, [activeTabId])

	function addTab() {
		const nextTab = createTab(tabs.length + 1, projectPath)
		setTabs((current) => [...current, nextTab])
		setActiveTabId(nextTab.id)
	}

	function closeTab(tabId: string) {
		tabApisRef.current.delete(tabId)
		setTabs((current) => {
			const next = current.filter((tab) => tab.id !== tabId)
			if (next.length === 0) {
				setTerminalOpen(false)
				return []
			}
			if (activeTabId === tabId) {
				const closedIndex = current.findIndex((tab) => tab.id === tabId)
				const fallback = next[Math.max(0, closedIndex - 1)] ?? next[0]
				setActiveTabId(fallback.id)
			}
			return next
		})
	}

	function handleResizePointerDown(event: React.PointerEvent<HTMLDivElement>) {
		event.preventDefault()
		setIsResizing(true)
		resizeStateRef.current = { startY: event.clientY, startHeight: height }

		function handlePointerMove(moveEvent: PointerEvent) {
			const state = resizeStateRef.current
			if (!state) return
			const delta = state.startY - moveEvent.clientY
			setHeight(clampTerminalHeight(state.startHeight + delta))
			fitActiveTab()
		}

		function handlePointerUp() {
			resizeStateRef.current = null
			setIsResizing(false)
			window.removeEventListener("pointermove", handlePointerMove)
			window.removeEventListener("pointerup", handlePointerUp)
		}

		window.addEventListener("pointermove", handlePointerMove)
		window.addEventListener("pointerup", handlePointerUp)
	}

	const motionTransition =
		isResizing || reduceMotion ? { duration: 0 } : terminalDrawer

	return (
		<motion.section
			data-slot="terminal-panel"
			className="relative z-20 flex shrink-0 flex-col overflow-hidden border-t border-border"
			style={{ backgroundColor: TERMINAL_SURFACE }}
			initial={reduceMotion ? false : { height: 0, opacity: 0 }}
			animate={{ height, opacity: 1 }}
			exit={{ height: 0, opacity: 0 }}
			transition={{
				height: motionTransition,
				opacity: motionTransition,
			}}
			onAnimationComplete={fitActiveTab}
			{...windowNoDragProps()}
		>
			<div
				role="separator"
				aria-orientation="horizontal"
				aria-label="Redimensionar terminal"
				onPointerDown={handleResizePointerDown}
				className="group flex h-1 shrink-0 cursor-row-resize items-center justify-center"
			>
				<div className="h-px w-8 rounded-full bg-border/80 transition-colors group-hover:bg-muted-foreground/60" />
			</div>

			<header className="flex h-7 shrink-0 items-center gap-1 border-b border-border/50 px-1.5">
				<TerminalSquare className="ml-0.5 size-3 shrink-0 text-muted-foreground" />

				<div className="scrollbar-thin flex min-w-0 flex-1 items-center gap-0.5 overflow-x-auto">
					{tabs.map((tab) => {
						const isActive = tab.id === activeTabId
						return (
							<div
								key={tab.id}
								className={cn(
									"group/tab flex h-5 max-w-[9rem] shrink-0 items-center rounded-sm border text-[11px] transition-colors",
									isActive
										? "border-border/80 bg-[rgba(255,255,255,0.08)] text-foreground"
										: "border-transparent text-muted-foreground hover:bg-[rgba(255,255,255,0.04)] hover:text-foreground",
								)}
							>
								<button
									type="button"
									onClick={() => setActiveTabId(tab.id)}
									className="min-w-0 flex-1 truncate px-2 text-left leading-none"
									title={tab.title}
								>
									{tab.title}
								</button>
								<button
									type="button"
									onClick={() => closeTab(tab.id)}
									className={cn(
										"mr-0.5 flex size-4 shrink-0 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-[rgba(255,255,255,0.08)] hover:text-foreground",
										!isActive && "opacity-0 group-hover/tab:opacity-100",
									)}
									aria-label={`Cerrar ${tab.title}`}
								>
									<X className="size-2.5" />
								</button>
							</div>
						)
					})}
				</div>

				<button
					type="button"
					onClick={addTab}
					className="flex size-5 shrink-0 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-[rgba(255,255,255,0.06)] hover:text-foreground"
					aria-label="Nueva terminal"
					title="Nueva terminal"
				>
					<Plus className="size-3" />
				</button>

				<button
					type="button"
					onClick={() => setTerminalOpen(false)}
					className="ml-0.5 flex size-5 shrink-0 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-[rgba(255,255,255,0.06)] hover:text-foreground"
					aria-label="Ocultar terminal"
					title="Ocultar terminal (⌘J)"
				>
					<X className="size-3" />
				</button>
			</header>

			<div className="relative min-h-0 flex-1">
				{isTauri ? (
					tabs.map((tab) => (
						<TerminalTabPane
							key={tab.id}
							tabId={tab.id}
							active={tab.id === activeTabId}
							cwd={tab.cwd}
							onRegister={registerTabApi}
						/>
					))
				) : (
					<div className="flex h-full items-center justify-center px-4 text-sm text-muted-foreground">
						La terminal integrada solo está disponible en la app de escritorio.
					</div>
				)}
			</div>
		</motion.section>
	)
}