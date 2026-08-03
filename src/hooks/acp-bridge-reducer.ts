import type { Store } from "jotai/vanilla/store"
import { appendStreamToMessages, applySessionUpdate } from "@/lib/acp-parser"
import {
	activePermissionAtom,
	agentConnectedAtom,
	capabilitiesAtom,
	configOptionsAtom,
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
			payload: { projectPath: string; capabilities: AgentCapabilities }
	  }
	| {
			type: "session_ready"
			payload: {
				sessionId: string
				projectPath: string
				configOptions?: ConfigOption[]
				resume?: boolean
			}
	  }
	| { type: "progress"; payload: { message?: string } }
	| { type: "session_update"; payload: unknown }
	| { type: "permission_request"; payload: PermissionRequest }
	| {
			type: "config_options"
			payload: { sessionId?: string; configOptions: ConfigOption[] }
	  }
	| { type: "prompt_complete"; payload?: { sessionId?: string } }
	| { type: "error"; payload: { message: string; sessionId?: string } }
	| { type: "disconnected" }

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
			store.set(agentConnectedAtom, true)
			store.set(projectPathAtom, event.payload.projectPath)
			store.set(capabilitiesAtom, event.payload.capabilities)
			store.set(sessionStatusAtom, "idle")
			store.set(progressMessageAtom, null)
			store.set(errorMessageAtom, null)
			return
		case "session_ready":
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
				isDifferentSession(event.payload.sessionId, refs.sessionId.current)
			) {
				return
			}
			store.set(activePermissionAtom, event.payload)
			store.set(sessionStatusAtom, "awaiting_permission")
			return
		case "config_options":
			if (
				isDifferentSession(event.payload.sessionId, refs.sessionId.current)
			) {
				return
			}
			store.set(configOptionsAtom, event.payload.configOptions)
			return
		case "prompt_complete":
			if (
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
