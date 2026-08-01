import { useSetAtom, useAtomValue } from "jotai"
import { useEffect } from "react"
import { getDefaultChatsPath, getProjectStatus, openProject } from "@/lib/tauri"
import {
	agentConnectedAtom,
	errorMessageAtom,
	progressMessageAtom,
} from "@/stores/atoms"

/** Dedupe concurrent bootstrap (React Strict Mode + Rust eager warm). */
let bootstrapPromise: Promise<void> | null = null

async function warmDefaultAgent(): Promise<void> {
	// Prefer reusing agent already started from Tauri setup() (single-flight in Rust).
	// open_project will wait on the in-flight warm instead of killing it.
	const status = await getProjectStatus()
	if (status.connected) {
		return
	}
	const chatsPath = await getDefaultChatsPath()
	// Agent only (initialize). session/new is New Chat — ACP session-setup.
	await openProject(chatsPath)
}

/**
 * Kick agent warm without blocking the UI.
 * OpenCode cold start is ~15–20s on the agent process — we never hold the shell on that.
 * `agent:ready` (via bridge) flips connected when initialize finishes.
 */
export function useBootstrapAgent() {
	const connected = useAtomValue(agentConnectedAtom)
	const setError = useSetAtom(errorMessageAtom)
	const setProgress = useSetAtom(progressMessageAtom)

	useEffect(() => {
		if (connected) {
			setProgress(null)
			return
		}

		// Non-blocking: open_project returns as soon as spawn is queued.
		if (!bootstrapPromise) {
			setProgress("Agent warming in background…")
			bootstrapPromise = warmDefaultAgent()
				.then(() => {
					// Stay quiet — agent:ready clears progress when initialize completes.
				})
				.catch((err) => {
					bootstrapPromise = null
					setError(
						err instanceof Error
							? err.message
							: "No se pudo iniciar el agente al arrancar",
					)
					setProgress(null)
				})
		}
	}, [connected, setError, setProgress])
}
