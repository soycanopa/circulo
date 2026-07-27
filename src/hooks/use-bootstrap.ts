import { useSetAtom } from "jotai"
import { useEffect, useRef } from "react"
import { getDefaultChatsPath, openProject } from "@/lib/tauri"
import {
	errorMessageAtom,
	sessionStatusAtom,
} from "@/stores/atoms"

/**
 * Warm OpenCode as soon as the app opens so General Chat / New Chat work
 * without waiting for the user to pick a project folder.
 */
export function useBootstrapAgent() {
	const setStatus = useSetAtom(sessionStatusAtom)
	const setError = useSetAtom(errorMessageAtom)
	const started = useRef(false)

	useEffect(() => {
		if (started.current) return
		started.current = true

		let cancelled = false

		void (async () => {
			setStatus("connecting")
			try {
				const chatsPath = await getDefaultChatsPath()
				if (cancelled) return
				await openProject(chatsPath)
			} catch (err) {
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
			cancelled = true
		}
	}, [setError, setStatus])
}
