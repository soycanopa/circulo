import { FitAddon } from "@xterm/addon-fit"
import { Terminal } from "@xterm/xterm"
import { useEffect, useRef } from "react"
import { spawn, type IPty } from "tauri-pty"
import { getDefaultShell } from "@/lib/terminal"

const TERMINAL_THEME = {
	background: "#141414",
	foreground: "#f5f5f5",
	cursor: "#6fcbf3",
	cursorAccent: "#141414",
	selectionBackground: "rgba(111, 203, 243, 0.28)",
	black: "#141414",
	red: "#fa423e",
	green: "#40c977",
	yellow: "#f5c542",
	blue: "#3b5ef9",
	magenta: "#c678dd",
	cyan: "#6fcbf3",
	white: "#f5f5f5",
	brightBlack: "#6b6b6b",
	brightRed: "#ff6764",
	brightGreen: "#5fd99b",
	brightYellow: "#ffd76b",
	brightBlue: "#6b8cff",
	brightMagenta: "#d8a6e8",
	brightCyan: "#8fd9f7",
	brightWhite: "#ffffff",
}

interface UseTerminalOptions {
	container: HTMLDivElement | null
	open: boolean
	cwd: string | null
}

export function useTerminal({ container, open, cwd }: UseTerminalOptions) {
	const terminalRef = useRef<Terminal | null>(null)
	const fitAddonRef = useRef<FitAddon | null>(null)
	const ptyRef = useRef<IPty | null>(null)
	const disposablesRef = useRef<Array<{ dispose: () => void }>>([])

	useEffect(() => {
		if (!open || !container) return

		const terminal = new Terminal({
			cursorBlink: true,
			fontFamily: 'ui-monospace, "SFMono-Regular", "SF Mono", Menlo, Consolas, monospace',
			fontSize: 12,
			lineHeight: 1.35,
			scrollback: 4000,
			theme: TERMINAL_THEME,
		})
		const fitAddon = new FitAddon()
		terminal.loadAddon(fitAddon)
		terminal.open(container)
		fitAddon.fit()

		const { file, args } = getDefaultShell()
		const pty = spawn(file, args, {
			cols: terminal.cols,
			rows: terminal.rows,
			cwd: cwd ?? undefined,
			name: "circulo-terminal",
		})

		const disposables: Array<{ dispose: () => void }> = []
		disposables.push(
			pty.onData((data) => {
				const text =
					data instanceof Uint8Array ? new TextDecoder().decode(data) : String(data)
				terminal.write(text)
			}),
		)
		disposables.push(
			pty.onExit(() => {
				terminal.write("\r\n\x1b[38;2;175;175;175m[proceso terminado]\x1b[0m\r\n")
			}),
		)
		disposables.push(
			terminal.onData((data) => {
				pty.write(data)
			}),
		)

		const resizeObserver = new ResizeObserver(() => {
			fitAddon.fit()
			pty.resize(terminal.cols, terminal.rows)
		})
		resizeObserver.observe(container)

		terminalRef.current = terminal
		fitAddonRef.current = fitAddon
		ptyRef.current = pty
		disposablesRef.current = disposables

		const focusTimeout = window.setTimeout(() => terminal.focus(), 0)

		return () => {
			window.clearTimeout(focusTimeout)
			resizeObserver.disconnect()
			for (const disposable of disposables) disposable.dispose()
			pty.kill()
			terminal.dispose()
			terminalRef.current = null
			fitAddonRef.current = null
			ptyRef.current = null
			disposablesRef.current = []
		}
	}, [open, container, cwd])

	return {
		fit: () => {
			fitAddonRef.current?.fit()
			const terminal = terminalRef.current
			const pty = ptyRef.current
			if (terminal && pty) pty.resize(terminal.cols, terminal.rows)
		},
		focus: () => terminalRef.current?.focus(),
	}
}