import { useAtom, useAtomValue } from "jotai"
import { TerminalSquare, X } from "lucide-react"
import { useCallback, useRef, useState } from "react"
import { useTerminal } from "@/hooks/use-terminal"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { clampTerminalHeight } from "@/lib/terminal"
import { isTauri } from "@/lib/window-chrome"
import { cn } from "@/lib/utils"
import { projectPathAtom, terminalHeightAtom, terminalOpenAtom } from "@/stores/atoms"
import "@xterm/xterm/css/xterm.css"

export function TerminalPanel() {
	const projectPath = useAtomValue(projectPathAtom)
	const [height, setHeight] = useAtom(terminalHeightAtom)
	const [, setTerminalOpen] = useAtom(terminalOpenAtom)
	const containerRef = useRef<HTMLDivElement>(null)
	const [container, setContainer] = useState<HTMLDivElement | null>(null)
	const resizeStateRef = useRef<{ startY: number; startHeight: number } | null>(null)

	const setContainerRef = useCallback((node: HTMLDivElement | null) => {
		containerRef.current = node
		setContainer(node)
	}, [])

	const { fit } = useTerminal({
		container,
		open: true,
		cwd: projectPath,
	})

	function handleResizePointerDown(event: React.PointerEvent<HTMLDivElement>) {
		event.preventDefault()
		resizeStateRef.current = { startY: event.clientY, startHeight: height }

		function handlePointerMove(moveEvent: PointerEvent) {
			const state = resizeStateRef.current
			if (!state) return
			const delta = state.startY - moveEvent.clientY
			const nextHeight = clampTerminalHeight(state.startHeight + delta)
			setHeight(nextHeight)
			fit()
		}

		function handlePointerUp() {
			resizeStateRef.current = null
			window.removeEventListener("pointermove", handlePointerMove)
			window.removeEventListener("pointerup", handlePointerUp)
		}

		window.addEventListener("pointermove", handlePointerMove)
		window.addEventListener("pointerup", handlePointerUp)
	}

	return (
		<section
			data-slot="terminal-panel"
			className="relative z-20 flex shrink-0 flex-col overflow-hidden border-t border-border bg-[#141414]"
			style={{ height }}
			{...windowNoDragProps()}
		>
			<div
				role="separator"
				aria-orientation="horizontal"
				aria-label="Redimensionar terminal"
				onPointerDown={handleResizePointerDown}
				className="group flex h-2 shrink-0 cursor-row-resize items-center justify-center border-b border-border/70"
			>
				<div className="h-px w-10 rounded-full bg-border transition-colors group-hover:bg-muted-foreground/70" />
			</div>

			<header className="flex shrink-0 items-center justify-between gap-2 border-b border-border/60 px-3 py-1.5">
				<div className="flex min-w-0 items-center gap-2 text-xs text-muted-foreground">
					<TerminalSquare className="size-3.5 shrink-0" />
					<span className="font-medium text-foreground">Terminal</span>
					{projectPath ? (
						<span className="truncate text-[11px]">{projectPath}</span>
					) : (
						<span className="text-[11px]">Sin proyecto</span>
					)}
				</div>
				<button
					type="button"
					onClick={() => setTerminalOpen(false)}
					className="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
					aria-label="Cerrar terminal"
				>
					<X className="size-3.5" />
				</button>
			</header>

			{isTauri ? (
				<div
					ref={setContainerRef}
					className={cn("min-h-0 flex-1 overflow-hidden px-1 py-1", "[&_.xterm]:h-full")}
				/>
			) : (
				<div className="flex flex-1 items-center justify-center px-4 text-sm text-muted-foreground">
					La terminal integrada solo está disponible en la app de escritorio.
				</div>
			)}
		</section>
	)
}