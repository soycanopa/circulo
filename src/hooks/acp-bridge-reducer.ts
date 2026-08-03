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
	progressMessageAtom,
	projectPathAtom,
	promptInFlightAtom,
	sessionIdAtom,
	sessionStatusAtom,
	streamingTextAtom,
} from "@/stores/atoms"
import type {
	AgentCapabilities,
	ConfigOption,
	PermissionRequest,
} from "@/types/acp"

export interface AcpBridgeRefs {
	streaming: { current: string }
	sessionId: { current: string | null }
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
	eventSessionId: string | undefined | null,
	activeSessionId: string | null,
) {
	return Boolean(
		eventSessionId && activeSessionId && eventSessionId !== activeSessionId,
	)
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
		case "session_ready":
			if (
				store.get(connectionGenerationAtom) !== null &&
				store.get(connectionGenerationAtom) !== event.payload.connectionGeneration
			) {
				return
			}
			store.set(connectionGenerationAtom, event.payload.connectionGeneration)
			refs.sessionId.current = event.payload.sessionId
			refs.firstChunkLogged.current = false
			store.set(sessionIdAtom, event.payload.sessionId)
			store.set(historyViewSessionIdAtom, null)
			store.set(projectPathAtom, event.payload.projectPath)
			store.set(configOptionsAtom, event.payload.configOptions ?? [])
			if (!event.payload.resume) {
				store.set(messagesAtom, [])
				refs.streaming.current = ""
				store.set(streamingTextAtom, "")
			}
			store.set(promptInFlightAtom, false)
			store.set(activePermissionAtom, null)
			store.set(sessionStatusAtom, "idle")
			store.set(progressMessageAtom, null)
			store.set(errorMessageAtom, null)
			return
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
			if (isDifferentSession(eventSessionId, refs.sessionId.current)) return

			const result = applySessionUpdate(
				store.get(messagesAtom),
				refs.streaming.current,
				event.payload,
			)
			refs.streaming.current = result.streamingText
			store.set(messagesAtom, result.messages)
			store.set(streamingTextAtom, result.streamingText)
			if (result.didStream) {
				store.set(sessionStatusAtom, "generating")
				store.set(promptInFlightAtom, true)
				if (!refs.firstChunkLogged.current) {
					refs.firstChunkLogged.current = true
					store.set(progressMessageAtom, null)
				}
			}
			return
		}
		case "permission_request":
			if (
				isStaleGeneration(store, event.payload.connectionGeneration) ||
				isDifferentSession(event.payload.sessionId, refs.sessionId.current)
			) {
				return
			}
			store.set(activePermissionAtom, event.payload)
			store.set(sessionStatusAtom, "awaiting_permission")
			return
		case "config_options":
			if (
				isStaleGeneration(store, event.payload.connectionGeneration) ||
				isDifferentSession(event.payload.sessionId, refs.sessionId.current)
			) {
				return
			}
			store.set(configOptionsAtom, event.payload.configOptions)
			return
		case "prompt_complete":
			if (
				isStaleGeneration(store, event.payload?.connectionGeneration) ||
				isDifferentSession(event.payload?.sessionId, refs.sessionId.current)
			) {
				return
			}
			store.set(
				messagesAtom,
				appendStreamToMessages(
					store.get(messagesAtom),
					refs.streaming.current,
				),
			)
			refs.streaming.current = ""
			refs.firstChunkLogged.current = false
			store.set(streamingTextAtom, "")
			store.set(promptInFlightAtom, false)
			store.set(sessionStatusAtom, "idle")
			store.set(activePermissionAtom, null)
			store.set(progressMessageAtom, null)
			return
		case "error":
			if (
				isStaleGeneration(store, event.payload.connectionGeneration) ||
				isDifferentSession(event.payload.sessionId, refs.sessionId.current)
			) {
				return
			}
			store.set(errorMessageAtom, event.payload.message)
			store.set(promptInFlightAtom, false)
			store.set(sessionStatusAtom, "idle")
			refs.streaming.current = ""
			store.set(streamingTextAtom, "")
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
			refs.sessionId.current = null
			store.set(projectPathAtom, null)
			store.set(sessionStatusAtom, "disconnected")
			store.set(promptInFlightAtom, false)
			store.set(activePermissionAtom, null)
			store.set(configOptionsAtom, [])
			store.set(capabilitiesAtom, null)
			store.set(progressMessageAtom, null)
	}
}
