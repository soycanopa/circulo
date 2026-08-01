import { useSetAtom, useAtomValue } from "jotai"
import { useEffect } from "react"
import {
	checkOpencode,
	getDefaultChatsPath,
	getProjectStatus,
	openProject,
} from "@/lib/tauri"
import {
	agentConnectedAtom,
	errorMessageAtom,
	opencodeStatusAtom,
	progressMessageAtom,
} from "@/stores/atoms"

/** Dedupe concurrent bootstrap (React Strict Mode + Rust eager warm). */
let bootstrapPromise: Promise<void> | null = null

async function warmDefaultAgent(): Promise<void> {
	const opencode = await checkOpencode()
	if (!opencode.available) {
		return
	}

	const status = await getProjectStatus()
	if (status.connected) {
		return
	}
	const chatsPath = await getDefaultChatsPath()
	await openProject(chatsPath)
}

/**
 * Verify OpenCode is installed, then kick agent warm without blocking the UI.
 */
export function useBootstrapAgent() {
	const connected = useAtomValue(agentConnectedAtom)
	const setError = useSetAtom(errorMessageAtom)
	const setProgress = useSetAtom(progressMessageAtom)
	const setOpencodeStatus = useSetAtom(opencodeStatusAtom)

	useEffect(() => {
		if (connected) {
			setProgress(null)
			return
		}

		if (!bootstrapPromise) {
			setProgress("Checking OpenCode…")
			bootstrapPromise = checkOpencode()
				.then((status) => {
					setOpencodeStatus(status)
					if (!status.available) {
						setProgress(null)
						return
					}
					setProgress("Agent warming in background…")
					return warmDefaultAgent()
				})
				.then(() => {
					// agent:ready clears progress when initialize completes.
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
	}, [connected, setError, setOpencodeStatus, setProgress])
}
