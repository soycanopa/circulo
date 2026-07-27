import { useSetAtom } from "jotai"
import { useEffect } from "react"
import { getDefaultChatsPath, openProject } from "@/lib/tauri"
import { errorMessageAtom, sessionStatusAtom } from "@/stores/atoms"

/** Dedupe concurrent bootstrap (React Strict Mode remounts the effect). */
let bootstrapPromise: Promise<void> | null = null

async function warmDefaultAgent(): Promise<void> {
	const chatsPath = await getDefaultChatsPath()
	await openProject(chatsPath)
}

/**
 * Warm OpenCode as soon as the app opens so General Chat works without
 * waiting for the user to pick a project folder.
 */
export function useBootstrapAgent() {
	const setStatus = useSetAtom(sessionStatusAtom)
	const setError = useSetAtom(errorMessageAtom)

	useEffect(() => {
		let cancelled = false

		void (async () => {
			setStatus("connecting")
			try {
				if (!bootstrapPromise) {
					bootstrapPromise = warmDefaultAgent().finally(() => {
						// Keep resolved promise so remounts reuse success; clear on failure so retry works.
					})
				}
				await bootstrapPromise
				// session:ready sets idle + sessionId; if still connecting, leave it —
				// openProject only resolves after session is ready.
			} catch (err) {
				bootstrapPromise = null
				if (cancelled) return
				setError(
					err instanceof Error
						? err.message
						: "No se pudo iniciar el agente al arrancar",
				)
				setStatus("idle")
			}
		})()

		return () => {
			// Do not cancel the in-flight warm — only ignore late UI updates via cancelled
			// for error handling. In-flight openProject must finish so the agent stays warm.
			cancelled = true
		}
	}, [setError, setStatus])
}
