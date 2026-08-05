import type { Store } from "jotai/vanilla/store"
import { getDefaultStore } from "jotai"
import { getProjectStatus } from "@/lib/tauri"
import {
	activeSessionIdAtom,
	agentConnectedAtom,
	capabilitiesAtom,
	configOptionsAtom,
	connectionGenerationAtom,
	historyMessagesAtom,
	historyViewSessionIdAtom,
	projectPathAtom,
	sessionsAtom,
	type SessionUiState,
} from "@/stores/atoms"
import type { ProjectStatus } from "@/types/acp"

const EMPTY_SESSION: SessionUiState = {
	messages: [],
	streaming: "",
	promptInFlight: false,
	status: "idle",
	configOptions: [],
	contextUsage: null,
}

function ensureSessionSlot(store: Store, sessionId: string) {
	const sessions = store.get(sessionsAtom)
	if (sessions[sessionId]) return
	store.set(sessionsAtom, {
		...sessions,
		[sessionId]: { ...EMPTY_SESSION },
	})
}

/**
 * Apply Rust `ProjectStatus` after a session command returns. Authoritative for
 * the invoke that produced this status (no stale-generation guard).
 */
export function reconcileSessionFromProjectStatus(
	store: Store,
	status: ProjectStatus,
): void {
	if (status.connectionGeneration !== null) {
		store.set(connectionGenerationAtom, status.connectionGeneration)
	}
	store.set(agentConnectedAtom, status.connected)
	if (status.projectPath) store.set(projectPathAtom, status.projectPath)
	if (status.capabilities) store.set(capabilitiesAtom, status.capabilities)

	if (status.sessionId) {
		store.set(activeSessionIdAtom, status.sessionId)
		store.set(historyViewSessionIdAtom, null)
		store.set(historyMessagesAtom, [])
		ensureSessionSlot(store, status.sessionId)
		if (status.configOptions.length) {
			store.set(configOptionsAtom, status.configOptions)
			store.set(sessionsAtom, (prev) => {
				const current = prev[status.sessionId!] ?? { ...EMPTY_SESSION }
				return {
					...prev,
					[status.sessionId!]: {
						...current,
						configOptions: status.configOptions,
					},
				}
			})
		}
	} else if (status.configOptions.length) {
		store.set(configOptionsAtom, status.configOptions)
	}
}

/** Reconcile from `getProjectStatus` when bootstrap catches up to an in-flight warm. */
export async function reconcileFromStatus(store?: Store): Promise<void> {
	const status = await getProjectStatus()
	const target = store ?? getDefaultStore()
	const currentGeneration = target.get(connectionGenerationAtom)
	if (
		!status.connected ||
		status.connectionGeneration === null ||
		(status.connectionGeneration !== undefined &&
			currentGeneration !== null &&
			status.connectionGeneration !== currentGeneration)
	) {
		return
	}
	reconcileSessionFromProjectStatus(target, status)
}
