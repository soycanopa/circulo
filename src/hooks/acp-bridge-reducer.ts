import type { Store } from "jotai/vanilla/store"
import { appendStreamToMessages, applySessionUpdate } from "@/lib/acp-parser"
import {
	activePermissionAtom,
	agentConnectedAtom,
	capabilitiesAtom,
	configOptionsAtom,
	connectionGenerationAtom,
	errorMessageAtom,
	historyViewSessionIdAtom,
	messagesAtom,
	pendingPermissionsAtom,
	progressMessageAtom,
	projectPathAtom,
	promptInFlightAtom,
	sessionIdAtom,
	sessionStatusAtom,
	sessionsAtom,
	streamingTextAtom,
	visibleSessionIdAtom,
	type SessionUiState,
} from "@/stores/atoms"
import type {
	AgentCapabilities,
	ConfigOption,
	PermissionRequest,
} from "@/types/acp"

export interface AcpBridgeRefs {
	streaming: { current: string }
	firstChunkLogged: { current: boolean }
}

type AcpBridgeEvent =
	| {
			type: "agent_ready"
			payload: {
				projectPath: string
				capabilities: AgentCapabilities
				connectionGeneration: number
			}
	  }
	| {
			type: "session_ready"
			payload: {
				sessionId: string
				projectPath: string
				configOptions?: ConfigOption[]
				connectionGeneration: number
				resume?: boolean
			}
	  }
	| { type: "progress"; payload: { message?: string } }
	| { type: "session_update"; payload: unknown }
	| { type: "permission_request"; payload: PermissionRequest }
	| {
			type: "config_options"
			payload: {
				sessionId?: string
				configOptions: ConfigOption[]
				connectionGeneration?: number
			}
	  }
	| {
			type: "prompt_complete"
			payload?: { sessionId?: string; connectionGeneration?: number }
	  }
	| {
			type: "error"
			payload: {
				message: string
				sessionId?: string
				connectionGeneration?: number
			}
	  }
	| { type: "disconnected"; payload: { connectionGeneration?: number } }

function isStaleGeneration(
	store: Store,
	generation: number | undefined,
): boolean {
	const current = store.get(connectionGenerationAtom)
	return generation !== undefined && current !== null && generation !== current
}

function isDifferentSession(
	store: Store,
	eventSessionId: string | undefined | null,
) {
	const activeSessionId =
		store.get(visibleSessionIdAtom) ?? store.get(sessionIdAtom)
	return (
		!activeSessionId || !eventSessionId || eventSessionId !== activeSessionId
	)
}

const EMPTY_SESSION: SessionUiState = {
	messages: [],
	streaming: "",
	promptInFlight: false,
	status: "idle",
	configOptions: [],
}

function ensureSession(store: Store, sessionId: string): SessionUiState {
	const sessions = store.get(sessionsAtom)
	const existing = sessions[sessionId]
	if (existing) return existing
	const next = { ...EMPTY_SESSION }
	store.set(sessionsAtom, { ...sessions, [sessionId]: next })
	return next
}

function updateSession(
	store: Store,
	sessionId: string,
	patch: Partial<SessionUiState>,
) {
	const sessions = store.get(sessionsAtom)
	const current = sessions[sessionId] ?? EMPTY_SESSION
	store.set(sessionsAtom, {
		...sessions,
		[sessionId]: { ...current, ...patch },
	})
	// Mirror the visible session into the legacy atoms so existing UI keeps working.
	if (store.get(visibleSessionIdAtom) === sessionId) {
		if (patch.messages !== undefined) store.set(messagesAtom, patch.messages)
		if (patch.streaming !== undefined) store.set(streamingTextAtom, patch.streaming)
		if (patch.promptInFlight !== undefined)
			store.set(promptInFlightAtom, patch.promptInFlight)
		if (patch.configOptions !== undefined)
			store.set(configOptionsAtom, patch.configOptions)
		if (patch.status !== undefined) store.set(sessionStatusAtom, patch.status)
	}
}

export function processAcpEvent(
	store: Store,
	refs: AcpBridgeRefs,
	event: AcpBridgeEvent,
) {
	switch (event.type) {
		case "agent_ready":
			store.set(connectionGenerationAtom, event.payload.connectionGeneration)
			store.set(agentConnectedAtom, true)
			store.set(projectPathAtom, event.payload.projectPath)
			store.set(capabilitiesAtom, event.payload.capabilities)
			store.set(sessionStatusAtom, "idle")
			store.set(progressMessageAtom, null)
			store.set(errorMessageAtom, null)
			return
		case "session_ready": {
			if (
				store.get(connectionGenerationAtom) !== null &&
				store.get(connectionGenerationAtom) !== event.payload.connectionGeneration
			) {
				return
			}
			store.set(connectionGenerationAtom, event.payload.connectionGeneration)
			refs.firstChunkLogged.current = false
			store.set(sessionIdAtom, event.payload.sessionId)
			store.set(visibleSessionIdAtom, event.payload.sessionId)
			store.set(historyViewSessionIdAtom, null)
			store.set(projectPathAtom, event.payload.projectPath)
			ensureSession(store, event.payload.sessionId)
			if (!event.payload.resume) {
				updateSession(store, event.payload.sessionId, {
					messages: [],
					streaming: "",
				})
				refs.streaming.current = ""
			}
			updateSession(store, event.payload.sessionId, {
				configOptions: event.payload.configOptions ?? [],
				promptInFlight: false,
				status: "idle",
			})
			store.set(activePermissionAtom, null)
			store.set(pendingPermissionsAtom, [])
			store.set(progressMessageAtom, null)
			store.set(errorMessageAtom, null)
			return
		}
		case "progress":
			if (event.payload.message) {
				store.set(progressMessageAtom, event.payload.message)
			}
			return
		case "session_update": {
			const root = event.payload as Record<string, unknown>
			const eventGeneration =
				typeof root.connectionGeneration === "number"
					? root.connectionGeneration
					: undefined
			if (isStaleGeneration(store, eventGeneration)) return
			const eventSessionId =
				(typeof root.sessionId === "string" && root.sessionId) ||
				(typeof root.session_id === "string" && root.session_id) ||
				null
			if (!eventSessionId) return
			if (isDifferentSession(store, eventSessionId)) return

			const sessionState = ensureSession(store, eventSessionId)
			const result = applySessionUpdate(
				sessionState.messages,
				refs.streaming.current,
				event.payload,
			)
			refs.streaming.current = result.streamingText
			updateSession(store, eventSessionId, {
				messages: result.messages,
				streaming: result.streamingText,
			})
			if (result.didStream) {
				updateSession(store, eventSessionId, {
					promptInFlight: true,
					status: "generating",
				})
				if (!refs.firstChunkLogged.current) {
					refs.firstChunkLogged.current = true
					store.set(progressMessageAtom, null)
				}
			}
			return
		}
		case "permission_request": {
			if (
				isStaleGeneration(store, event.payload.connectionGeneration) ||
				isDifferentSession(store, event.payload.sessionId)
			) {
				return
			}
			store.set(pendingPermissionsAtom, (prev) => {
				const next = [...prev, event.payload]
				store.set(activePermissionAtom, next[0] ?? null)
				if (event.payload.sessionId) {
					updateSession(store, event.payload.sessionId, {
						status: "awaiting_permission",
					})
				} else {
					store.set(sessionStatusAtom, "awaiting_permission")
				}
				return next
			})
			return
		}		case "config_options":
			if (
				isStaleGeneration(store, event.payload.connectionGeneration) ||
				isDifferentSession(store, event.payload.sessionId)
			) {
				return
			}
			if (event.payload.sessionId) {
				updateSession(store, event.payload.sessionId, {
					configOptions: event.payload.configOptions,
				})
			} else {
				store.set(configOptionsAtom, event.payload.configOptions)
			}
			return
		case "prompt_complete": {
			if (
				isStaleGeneration(store, event.payload?.connectionGeneration) ||
				isDifferentSession(store, event.payload?.sessionId)
			) {
				return
			}
			const sid = event.payload?.sessionId
			if (sid) {
				const state = ensureSession(store, sid)
				const next = appendStreamToMessages(state.messages, refs.streaming.current)
				updateSession(store, sid, {
					messages: next,
					streaming: "",
					promptInFlight: false,
					status: "idle",
				})
			} else {
				store.set(
					messagesAtom,
					appendStreamToMessages(
						store.get(messagesAtom),
						refs.streaming.current,
					),
				)
				store.set(streamingTextAtom, "")
				store.set(promptInFlightAtom, false)
				store.set(sessionStatusAtom, "idle")
			}
			refs.streaming.current = ""
			refs.firstChunkLogged.current = false
			store.set(activePermissionAtom, null)
			store.set(pendingPermissionsAtom, [])
			store.set(progressMessageAtom, null)
			return
		}
		case "error":
			if (
				isStaleGeneration(store, event.payload.connectionGeneration) ||
				isDifferentSession(store, event.payload.sessionId)
			) {
				return
			}
			store.set(errorMessageAtom, event.payload.message)
			const errSid = event.payload.sessionId
			if (errSid) {
				updateSession(store, errSid, {
					promptInFlight: false,
					status: "idle",
					streaming: "",
				})
			} else {
				store.set(promptInFlightAtom, false)
				store.set(sessionStatusAtom, "idle")
				store.set(streamingTextAtom, "")
			}
			refs.streaming.current = ""
			store.set(progressMessageAtom, null)
			return
		case "disconnected":
			if (
				event.payload.connectionGeneration !== undefined &&
				store.get(connectionGenerationAtom) !== null &&
				event.payload.connectionGeneration !==
					store.get(connectionGenerationAtom)
			) {
				return
			}
			store.set(connectionGenerationAtom, null)
			store.set(agentConnectedAtom, false)
			store.set(sessionIdAtom, null)
			store.set(visibleSessionIdAtom, null)
			store.set(sessionsAtom, {})
			store.set(projectPathAtom, null)
			store.set(sessionStatusAtom, "disconnected")
			store.set(promptInFlightAtom, false)
			store.set(messagesAtom, [])
			store.set(streamingTextAtom, "")
			store.set(activePermissionAtom, null)
			store.set(pendingPermissionsAtom, [])
			store.set(configOptionsAtom, [])
			store.set(capabilitiesAtom, null)
			store.set(progressMessageAtom, null)
	}
}
