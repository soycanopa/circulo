import { FitAddon } from "@xterm/addon-fit"
import { Terminal } from "@xterm/xterm"
import { useEffect, useRef } from "react"
import {
	registerTerminalWriter,
	unregisterTerminalWriter,
} from "@/lib/terminal-output-bridge"
import {
	closeUserTerminal,
	resizeUserTerminal,
	spawnUserTerminal,
	writeUserTerminal,
} from "@/lib/tauri"
import "@xterm/xterm/css/xterm.css"

const CHAT_CONTENT_BG = "#151516"
const CHAT_FG = "#f3f3f3"

interface EmbeddedTerminalProps {
	tabId: string
	projectPath: string | null
	isActive: boolean
}

const tabRefCounts = new Map<string, number>()
const pendingCloses = new Map<string, ReturnType<typeof setTimeout>>()

function cancelPendingClose(tabId: string) {
	const timer = pendingCloses.get(tabId)
	if (timer) {
		clearTimeout(timer)
		pendingCloses.delete(tabId)
	}
}

function schedulePendingClose(tabId: string) {
	cancelPendingClose(tabId)
	pendingCloses.set(
		tabId,
		setTimeout(() => {
			pendingCloses.delete(tabId)
			void closeUserTerminal(tabId)
		}, 200),
	)
}

function acquireTab(tabId: string) {
	const next = (tabRefCounts.get(tabId) ?? 0) + 1
	tabRefCounts.set(tabId, next)
	cancelPendingClose(tabId)
}

function releaseTab(tabId: string) {
	const next = (tabRefCounts.get(tabId) ?? 1) - 1
	if (next <= 0) {
		tabRefCounts.delete(tabId)
		schedulePendingClose(tabId)
	} else {
		tabRefCounts.set(tabId, next)
	}
}

async function waitForContainerSize(
	container: HTMLElement,
	fitAddon: FitAddon,
	term: Terminal,
): Promise<{ cols: number; rows: number }> {
	for (let attempt = 0; attempt < 30; attempt += 1) {
		fitAddon.fit()
		if (container.clientWidth > 0 && container.clientHeight > 0) {
			return {
				cols: Math.max(term.cols, 2),
				rows: Math.max(term.rows, 2),
			}
		}
		await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()))
	}
	fitAddon.fit()
	return { cols: Math.max(term.cols, 80), rows: Math.max(term.rows, 24) }
}

export function EmbeddedTerminal({
	tabId,
	projectPath,
	isActive,
}: EmbeddedTerminalProps) {
	const containerRef = useRef<HTMLDivElement>(null)
	const termRef = useRef<Terminal | null>(null)
	const fitRef = useRef<FitAddon | null>(null)

	useEffect(() => {
		acquireTab(tabId)

		const container = containerRef.current
		if (!container || !projectPath) {
			return () => {
				releaseTab(tabId)
			}
		}

		const term = new Terminal({
			cursorBlink: true,
			convertEol: true,
			fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
			fontSize: 12,
			theme: {
				background: CHAT_CONTENT_BG,
				foreground: CHAT_FG,
				cursor: CHAT_FG,
				selectionBackground: "rgba(255, 255, 255, 0.15)",
			},
			scrollback: 5000,
		})
		const fitAddon = new FitAddon()
		term.loadAddon(fitAddon)
		term.open(container)
		termRef.current = term
		fitRef.current = fitAddon

		let disposed = false

		const syncSize = () => {
			fitAddon.fit()
			const cols = Math.max(term.cols, 2)
			const rows = Math.max(term.rows, 2)
			void resizeUserTerminal(tabId, cols, rows).catch(() => {})
			return { cols, rows }
		}

		void (async () => {
			try {
				await registerTerminalWriter(tabId, term)
				if (disposed) return

				const { cols, rows } = await waitForContainerSize(container, fitAddon, term)
				if (disposed) return

				await spawnUserTerminal(tabId, projectPath, cols, rows)
				if (disposed) return

				syncSize()
				if (isActive) term.focus()
			} catch (err) {
				const message =
					err instanceof Error ? err.message : "Failed to start embedded terminal"
				term.writeln(`\x1b[38;5;203m${message}\x1b[0m`)
			}
		})()

		const onData = term.onData((data) => {
			void writeUserTerminal(tabId, data).catch(() => {})
		})

		const resizeObserver = new ResizeObserver(() => {
			if (!termRef.current) return
			syncSize()
		})
		resizeObserver.observe(container)

		return () => {
			disposed = true
			onData.dispose()
			resizeObserver.disconnect()
			unregisterTerminalWriter(tabId)
			term.dispose()
			termRef.current = null
			fitRef.current = null
			releaseTab(tabId)
		}
	}, [projectPath, tabId])

	useEffect(() => {
		if (!isActive) return
		const term = termRef.current
		const fitAddon = fitRef.current
		if (!term || !fitAddon) return
		requestAnimationFrame(() => {
			fitAddon.fit()
			void resizeUserTerminal(
				tabId,
				Math.max(term.cols, 2),
				Math.max(term.rows, 2),
			).catch(() => {})
			term.focus()
		})
	}, [isActive, tabId])

	if (!projectPath) {
		return (
			<div className="flex flex-1 items-center justify-center px-3 text-xs text-muted">
				Open a project to use the embedded terminal.
			</div>
		)
	}

	return (
		<div
			ref={containerRef}
			className="h-full min-h-0 w-full bg-content px-1 py-1"
			aria-hidden={!isActive}
		/>
	)
}

export function cancelEmbeddedTerminalClose(tabId: string) {
	cancelPendingClose(tabId)
}
