import { useEffect } from "react"
import { getDefaultStore } from "jotai"
import { listen } from "@tauri-apps/api/event"
import {
	connectionGenerationAtom,
	mcpSavingsAtom,
	terminalsAtom,
} from "@/stores/atoms"
import type { TerminalState } from "@/types/acp"

interface TerminalOutputPayload {
	terminalId: string
	sessionId: string
	label: string
	output: string
	truncated: boolean
	running: boolean
	exitStatus?: { exitCode?: number; signal?: string } | null
	connectionGeneration: number
}

interface TerminalFilterStatsPayload {
	terminalId: string
	sessionId: string
	originalBytes: number
	filteredBytes: number
	savedBytes: number
	connectionGeneration: number
}

export function useTerminalBridge() {
	useEffect(() => {
		let unlisten: (() => void) | undefined

		listen<TerminalOutputPayload>("acp:terminal_output", (event) => {
			const payload = event.payload
			const store = getDefaultStore()
			const currentGen = store.get(connectionGenerationAtom)
			if (
				currentGen !== null &&
				payload.connectionGeneration !== currentGen
			) {
				return
			}

			const next: TerminalState = {
				terminalId: payload.terminalId,
				sessionId: payload.sessionId,
				label: payload.label,
				output: payload.output,
				truncated: payload.truncated,
				running: payload.running,
				exitStatus: payload.exitStatus ?? null,
			}

			store.set(terminalsAtom, (prev) => ({
				...prev,
				[payload.terminalId]: next,
			}))
		}).then((fn) => {
			unlisten = fn
		})

		listen<TerminalFilterStatsPayload>(
			"acp:terminal_filter_stats",
			(event) => {
				const payload = event.payload
				const store = getDefaultStore()
				const currentGen = store.get(connectionGenerationAtom)
				if (
					currentGen !== null &&
					payload.connectionGeneration !== currentGen
				) {
					return
				}
				store.set(mcpSavingsAtom, (prev) => ({
					...prev,
					savingsBytes: prev.savingsBytes + payload.savedBytes,
					compactionCount: prev.compactionCount + 1,
				}))
			},
		).then((fn) => {
			unlisten = fn
		})

		return () => {
			unlisten?.()
		}
	}, [])
}
