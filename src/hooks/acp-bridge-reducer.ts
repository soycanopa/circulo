import type { Store } from "jotai/vanilla/store"
import { appendStreamToMessages, applySessionUpdate, parseUsageUpdate } from "@/lib/acp-parser"
import { reconcileFromStatus } from "@/hooks/session-reconcile"
import {
	activePermissionAtom,
	activeSessionIdAtom,
	agentConnectedAtom,
	capabilitiesAtom,
	configOptionsAtom,
	connectionGenerationAtom,
	errorMessageAtom,
	historyMessagesAtom,
	historyViewSessionIdAtom,
	pendingPermissionsAtom,
	progressMessageAtom,
	warmTimingsAtom,
	projectPathAtom,
	sessionStatusAtom,
	sessionsAtom,
	type SessionUiState,
} from "@/stores/atoms"
import type {
	AgentCapabilities,
	ConfigOption,
	PermissionRequest,
} from "@/types/acp"

export interface AcpBridgeRefs {
	streaming: { current: Map<string, string> }
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
	| {
			type: "visible_session_changed"
			payload: {
				sessionId: string | null
				configOptions?: ConfigOption[]
				connectionGeneration: number
			}
	  }
	| {
			type: "progress"
			payload: {
				phase?: string
				message?: string
				elapsedMs?: number
				connectionGeneration?: number
			}
	  }
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

/** True when the event refers to a session we have never seen on this agent. */
function isUnknownSession(
	store: Store,
	eventSessionId: string | undefined | null,
): boolean {
	if (!eventSessionId) return true
	const active = store.get(activeSessionIdAtom)
	if (active === eventSessionId) return false
	const sessions = store.get(sessionsAtom)
	return !sessions[eventSessionId]
}

const EMPTY_SESSION: SessionUiState = {
	messages: [],
	streaming: "",
	promptInFlight: false,
	status: "idle",
	configOptions: [],
	contextUsage: null,
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
}

function streamFor(refs: AcpBridgeRefs, sessionId: string): string {
	let value = refs.streaming.current.get(sessionId)
	if (value === undefined) {
		value = ""
		refs.streaming.current.set(sessionId, value)
	}
	return value
}

function setStreamFor(refs: AcpBridgeRefs, sessionId: string, value: string) {
	refs.streaming.current.set(sessionId, value)
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
			void reconcileFromStatus(store).catch(() => {
				// Non-fatal: invoke may be unavailable before listeners attach (tests).
			})
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
			// `activeSessionIdAtom` is the single source of truth for the visible session.
			store.set(activeSessionIdAtom, event.payload.sessionId)
			store.set(historyViewSessionIdAtom, null)
			store.set(historyMessagesAtom, [])
			store.set(projectPathAtom, event.payload.projectPath)
			ensureSession(store, event.payload.sessionId)
			if (!event.payload.resume) {
				updateSession(store, event.payload.sessionId, {
					messages: [],
					streaming: "",
					contextUsage: null,
				})
				setStreamFor(refs, event.payload.sessionId, "")
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
		case "visible_session_changed": {
			if (
				store.get(connectionGenerationAtom) !== null &&
				store.get(connectionGenerationAtom) !== event.payload.connectionGeneration
			) {
				return
			}
			store.set(connectionGenerationAtom, event.payload.connectionGeneration)
			store.set(historyViewSessionIdAtom, null)
			store.set(historyMessagesAtom, [])
			if (event.payload.sessionId) {
				store.set(activeSessionIdAtom, event.payload.sessionId)
				ensureSession(store, event.payload.sessionId)
				if (event.payload.configOptions?.length) {
					updateSession(store, event.payload.sessionId, {
						configOptions: event.payload.configOptions,
					})
					store.set(configOptionsAtom, event.payload.configOptions)
				}
			} else {
				store.set(activeSessionIdAtom, null)
			}
			return
		}
		case "progress":
			if (
				isStaleGeneration(store, event.payload.connectionGeneration)
			) {
				return
			}
			if (event.payload.message) {
				store.set(progressMessageAtom, event.payload.message)
			}
			if (event.payload.elapsedMs !== undefined) {
				const phase = event.payload.phase
				const elapsed = event.payload.elapsedMs
				store.set(warmTimingsAtom, (prev) => {
					if (phase === "initialize") {
						return { ...prev, initializeMs: elapsed }
					}
					if (phase === "session_prewarm") {
						return { ...prev, prewarmMs: elapsed }
					}
					if (phase === "config_refresh") {
						return { ...prev, configRefreshMs: elapsed }
					}
					return prev
				})
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
			if (isUnknownSession(store, eventSessionId)) return

			const sessionState = ensureSession(store, eventSessionId)
			const result = applySessionUpdate(
				sessionState.messages,
				streamFor(refs, eventSessionId),
				event.payload,
			)
			setStreamFor(refs, eventSessionId, result.streamingText)
			updateSession(store, eventSessionId, {
				messages: result.messages,
				streaming: result.streamingText,
			})
			const usage = parseUsageUpdate(event.payload)
			if (usage) {
				updateSession(store, eventSessionId, { contextUsage: usage })
			}
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
				isUnknownSession(store, event.payload.sessionId)
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
		}
		case "config_options":
			if (isStaleGeneration(store, event.payload.connectionGeneration)) {
				return
			}
			store.set(configOptionsAtom, event.payload.configOptions)
			if (event.payload.sessionId) {
				ensureSession(store, event.payload.sessionId)
				updateSession(store, event.payload.sessionId, {
					configOptions: event.payload.configOptions,
				})
			}
			return
		case "prompt_complete": {
			if (
				isStaleGeneration(store, event.payload?.connectionGeneration) ||
				isUnknownSession(store, event.payload?.sessionId)
			) {
				return
			}
			const sid = event.payload?.sessionId
			if (sid) {
				const state = ensureSession(store, sid)
				const next = appendStreamToMessages(
					state.messages,
					streamFor(refs, sid),
				)
				updateSession(store, sid, {
					messages: next,
					streaming: "",
					promptInFlight: false,
					status: "idle",
				})
			} else {
				// Fallback for events without session id — append to the currently
				// active session if any, otherwise drop.
				const active = store.get(activeSessionIdAtom)
				if (active) {
					const state = ensureSession(store, active)
					const next = appendStreamToMessages(
						state.messages,
						streamFor(refs, active),
					)
					updateSession(store, active, {
						messages: next,
						streaming: "",
						promptInFlight: false,
						status: "idle",
					})
				}
			}
			const completedSid = sid ?? store.get(activeSessionIdAtom)
			if (completedSid) {
				refs.streaming.current.delete(completedSid)
			}
			if (completedSid === store.get(activeSessionIdAtom)) {
				refs.firstChunkLogged.current = false
				store.set(activePermissionAtom, null)
				store.set(pendingPermissionsAtom, [])
			}
			store.set(progressMessageAtom, null)
			return
		}
		case "error":
			if (
				isStaleGeneration(store, event.payload.connectionGeneration) ||
				isUnknownSession(store, event.payload.sessionId)
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
				refs.streaming.current.delete(errSid)
			}
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
			store.set(activeSessionIdAtom, null)
			store.set(sessionsAtom, {})
			store.set(projectPathAtom, null)
			store.set(sessionStatusAtom, "disconnected")
			store.set(activePermissionAtom, null)
			store.set(pendingPermissionsAtom, [])
			store.set(configOptionsAtom, [])
			store.set(capabilitiesAtom, null)
			store.set(progressMessageAtom, null)
			store.set(warmTimingsAtom, {})
	}
}
