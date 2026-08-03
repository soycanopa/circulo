import { getDefaultStore, useSetAtom, useAtomValue } from "jotai"
import { useEffect } from "react"
import { checkOpencode, getDefaultChatsPath, getProjectStatus, openProject } from "@/lib/tauri"
import { reconcileFromStatus } from "@/hooks/session-reconcile"
import {
	agentConnectedAtom,
	errorMessageAtom,
	opencodeStatusAtom,
	progressMessageAtom,
} from "@/stores/atoms"

/** Resolves when the ACP event bus has installed its listeners (no race on first event). */
let listenersReady: Promise<void> | null = null

/** Dedupe concurrent bootstrap (React Strict Mode + Rust eager warm). */
let bootstrapPromise: Promise<void> | null = null

export function setListenersReady(promise: Promise<unknown>) {
	if (!listenersReady) {
		listenersReady = promise.then(() => undefined)
	}
}

export function waitForListeners(): Promise<void> {
	return listenersReady ?? Promise.resolve()
}

async function warmDefaultAgent(): Promise<void> {
	const opencode = await checkOpencode()
	if (!opencode.available) {
		return
	}

	await reconcileFromStatus(getDefaultStore())
	const status = await getProjectStatus()
	if (status.connected) {
		return
	}
	const chatsPath = await getDefaultChatsPath()
	await openProject(chatsPath)
	await reconcileFromStatus(getDefaultStore())
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
			bootstrapPromise = waitForListeners()
				.then(() => checkOpencode())
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
							: "Failed to start agent on launch",
					)
					setProgress(null)
				})
		}
	}, [connected, setError, setOpencodeStatus, setProgress])
}
