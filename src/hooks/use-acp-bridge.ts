import { getDefaultStore, useSetAtom } from "jotai"
import { useEffect, useRef } from "react"
import {
	appendStreamToMessages,
	applySessionUpdate,
} from "@/lib/acp-parser"
import { listenAcpEvents } from "@/lib/tauri"
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

/**
 * Single root registration for ACP events.
 * Call once from App root.
 *
 * Per ACP prompt-turn: session/update (agent_message_chunk) arrives *during*
 * session/prompt — paint immediately; do not wait for prompt_complete.
 */
export function useAcpBridge() {
	const setMessages = useSetAtom(messagesAtom)
	const setStreaming = useSetAtom(streamingTextAtom)
	const setSessionId = useSetAtom(sessionIdAtom)
	const setProjectPath = useSetAtom(projectPathAtom)
	const setConnected = useSetAtom(agentConnectedAtom)
	const setStatus = useSetAtom(sessionStatusAtom)
	const setPromptInFlight = useSetAtom(promptInFlightAtom)
	const setConfig = useSetAtom(configOptionsAtom)
	const setCapabilities = useSetAtom(capabilitiesAtom)
	const setPermission = useSetAtom(activePermissionAtom)
	const setError = useSetAtom(errorMessageAtom)
	const setProgress = useSetAtom(progressMessageAtom)
	const setHistoryView = useSetAtom(historyViewSessionIdAtom)

	const streamingRef = useRef("")
	const sessionIdRef = useRef<string | null>(null)
	const firstChunkLogged = useRef(false)

	useEffect(() => {
		let cancelled = false
		let unlisteners: Array<() => void> = []

		listenAcpEvents({
			onAgentReady: (payload) => {
				setConnected(true)
				setProjectPath(payload.projectPath)
				setCapabilities(payload.capabilities)
				setStatus("idle")
				setProgress(null)
				setError(null)
			},
			onSessionReady: (payload) => {
				sessionIdRef.current = payload.sessionId
				firstChunkLogged.current = false
				setSessionId(payload.sessionId)
				setHistoryView(null)
				setProjectPath(payload.projectPath)
				setConfig(payload.configOptions ?? [])
				setMessages([])
				streamingRef.current = ""
				setStreaming("")
				setPromptInFlight(false)
				setPermission(null)
				setStatus("idle")
				setProgress(null)
				setError(null)
			},
			onProgress: (payload) => {
				if (payload.message) setProgress(payload.message)
			},
			onSessionUpdate: (payload) => {
				const root = payload as Record<string, unknown>
				const eventSessionId =
					(typeof root.sessionId === "string" && root.sessionId) ||
					(typeof root.session_id === "string" && root.session_id) ||
					null
				if (
					eventSessionId &&
					sessionIdRef.current &&
					eventSessionId !== sessionIdRef.current
				) {
					return
				}

				const store = getDefaultStore()
				const current = store.get(messagesAtom)
				const result = applySessionUpdate(
					current,
					streamingRef.current,
					payload,
				)
				streamingRef.current = result.streamingText

				// Never nest setState inside another updater — paint stream immediately.
				store.set(messagesAtom, result.messages)
				store.set(streamingTextAtom, result.streamingText)

				if (result.didStream) {
					store.set(sessionStatusAtom, "generating")
					store.set(promptInFlightAtom, true)
					if (!firstChunkLogged.current) {
						firstChunkLogged.current = true
						store.set(progressMessageAtom, null)
					}
				}
			},
			onPermissionRequest: (payload) => {
				if (
					payload.sessionId &&
					sessionIdRef.current &&
					payload.sessionId !== sessionIdRef.current
				) {
					return
				}
				setPermission(payload)
				setStatus("awaiting_permission")
			},
			onConfigOptions: (payload) => {
				if (
					payload.sessionId &&
					sessionIdRef.current &&
					payload.sessionId !== sessionIdRef.current
				) {
					return
				}
				setConfig(payload.configOptions)
			},
			onPromptComplete: (payload) => {
				if (
					payload?.sessionId &&
					sessionIdRef.current &&
					payload.sessionId !== sessionIdRef.current
				) {
					return
				}
				const store = getDefaultStore()
				const current = store.get(messagesAtom)
				const next = appendStreamToMessages(current, streamingRef.current)
				streamingRef.current = ""
				firstChunkLogged.current = false
				store.set(messagesAtom, next)
				store.set(streamingTextAtom, "")
				store.set(promptInFlightAtom, false)
				store.set(sessionStatusAtom, "idle")
				store.set(activePermissionAtom, null)
				store.set(progressMessageAtom, null)
			},
			onError: (payload) => {
				if (
					payload.sessionId &&
					sessionIdRef.current &&
					payload.sessionId !== sessionIdRef.current
				) {
					return
				}
				setError(payload.message)
				setPromptInFlight(false)
				setStatus("idle")
				streamingRef.current = ""
				setStreaming("")
				setProgress(null)
			},
			onDisconnected: () => {
				setConnected(false)
				setSessionId(null)
				sessionIdRef.current = null
				setProjectPath(null)
				setStatus("disconnected")
				setPromptInFlight(false)
				setPermission(null)
				setConfig([])
				setCapabilities(null)
				setProgress(null)
			},
		}).then((list) => {
			if (cancelled) {
				for (const u of list) u()
				return
			}
			unlisteners = list
		})

		return () => {
			cancelled = true
			for (const u of unlisteners) u()
		}
	}, [
		setCapabilities,
		setConfig,
		setConnected,
		setError,
		setMessages,
		setPermission,
		setProjectPath,
		setPromptInFlight,
		setSessionId,
		setStatus,
		setStreaming,
		setProgress,
		setHistoryView,
	])
}
