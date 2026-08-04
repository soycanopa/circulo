import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import type { Terminal } from "@xterm/xterm"

interface TerminalOutputEvent {
	tabId: string
	data: string
}

const CLEAR_DISPLAY_RE = /\x1b\[[0-9;]*2J/

function writeWithClear(term: Terminal, data: string) {
	term.write(data)
	if (CLEAR_DISPLAY_RE.test(data)) {
		// Standard `clear` only erases the viewport; also drop scrollback.
		term.write("\x1b[3J")
	}
}

const writers = new Map<string, Terminal>()
const pendingOutput = new Map<string, string>()
let outputUnlisten: UnlistenFn | null = null
let outputReady: Promise<void> | null = null

function routeOutput(tabId: string, data: string) {
	const term = writers.get(tabId)
	if (term) {
		writeWithClear(term, data)
		return
	}
	const prev = pendingOutput.get(tabId) ?? ""
	pendingOutput.set(tabId, prev + data)
}

async function ensureOutputListener() {
	if (outputUnlisten) return
	if (!outputReady) {
		outputReady = listen<TerminalOutputEvent>("user_terminal_output", (event) => {
			routeOutput(event.payload.tabId, event.payload.data)
		}).then((unlisten) => {
			outputUnlisten = unlisten
		})
	}
	await outputReady
}

export async function registerTerminalWriter(
	tabId: string,
	term: Terminal,
): Promise<void> {
	await ensureOutputListener()
	writers.set(tabId, term)
	const buffered = pendingOutput.get(tabId)
	if (buffered) {
		writeWithClear(term, buffered)
		pendingOutput.delete(tabId)
	}
}

export function unregisterTerminalWriter(tabId: string) {
	writers.delete(tabId)
}

export async function ensureTerminalOutputListener(): Promise<void> {
	await ensureOutputListener()
}

export function clearTerminalOutputBuffer(tabId: string) {
	pendingOutput.delete(tabId)
}
