import { getDefaultStore, useSetAtom, useAtomValue } from "jotai"
import { useEffect } from "react"
import { getDefaultChatsPath, getProjectStatus, listAgents, openProject } from "@/lib/tauri"
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

async function warmDefaultAgents(): Promise<void> {
	const agents = await listAgents()
	const opencode = agents.find((a) => a.id === "opencode")
	if (opencode) {
		getDefaultStore().set(opencodeStatusAtom, {
			available: opencode.available,
			path: opencode.available ? opencode.command : null,
			installHint: opencode.available
				? ""
				: "Install OpenCode from https://opencode.ai or set OPENCODE_BIN to the full binary path.",
		})
	}

	const anyAvailable = agents.some((a) => a.available)
	if (!anyAvailable) {
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
 * Kick multi-agent warm without blocking the UI (Rust also warms the pool at startup).
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

		if (!bootstrapPromise) {
			setProgress("Preparando agentes…")
			bootstrapPromise = waitForListeners()
				.then(() => warmDefaultAgents())
				.then(() => {
					// agent:ready clears progress when the active agent finishes initialize.
				})
				.catch((err) => {
					bootstrapPromise = null
					setError(
						err instanceof Error
							? err.message
							: "Failed to start agents on launch",
					)
					setProgress(null)
				})
		}
	}, [connected, setError, setProgress])
}
