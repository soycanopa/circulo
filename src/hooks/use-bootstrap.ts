import { useSetAtom, useAtomValue } from "jotai"
import { useEffect } from "react"
import { getDefaultChatsPath, getProjectStatus, openProject } from "@/lib/tauri"
import {
	agentConnectedAtom,
	errorMessageAtom,
	progressMessageAtom,
	sessionStatusAtom,
} from "@/stores/atoms"

/** Dedupe concurrent bootstrap (React Strict Mode + Rust eager warm). */
let bootstrapPromise: Promise<void> | null = null

async function warmDefaultAgent(): Promise<void> {
	// Prefer reusing agent already started from Tauri setup().
	const status = await getProjectStatus()
	if (status.connected) {
		return
	}
	const chatsPath = await getDefaultChatsPath()
	await openProject(chatsPath)
}

/**
 * Ensure the agent process is warm. Does not create a chat session.
 * Session starts only via New Chat (or first message).
 */
export function useBootstrapAgent() {
	const connected = useAtomValue(agentConnectedAtom)
	const setStatus = useSetAtom(sessionStatusAtom)
	const setError = useSetAtom(errorMessageAtom)
	const setProgress = useSetAtom(progressMessageAtom)

	useEffect(() => {
		if (connected) return

		let cancelled = false

		void (async () => {
			setProgress("Warming agent in background…")
			try {
				if (!bootstrapPromise) {
					bootstrapPromise = warmDefaultAgent().catch((err) => {
						bootstrapPromise = null
						throw err
					})
				}
				await bootstrapPromise
				if (cancelled) return
				// agent:ready event sets connected; no session yet is correct.
				setProgress(null)
			} catch (err) {
				if (cancelled) return
				setError(
					err instanceof Error
						? err.message
						: "No se pudo iniciar el agente al arrancar",
				)
				setStatus("idle")
				setProgress(null)
			}
		})()

		return () => {
			cancelled = true
		}
	}, [connected, setError, setProgress, setStatus])
}
