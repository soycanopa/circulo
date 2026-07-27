import { useSetAtom } from "jotai"
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
 * Call once from AppShell / App root.
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

	const streamingRef = useRef("")
	const sessionIdRef = useRef<string | null>(null)

	useEffect(() => {
		let cancelled = false
		let unlisteners: Array<() => void> = []

		listenAcpEvents({
			onAgentReady: (payload) => {
				setConnected(true)
				setProjectPath(payload.projectPath)
				setCapabilities(payload.capabilities)
				// Agent warm only — no session yet.
				setStatus("idle")
				setProgress(null)
				setError(null)
			},
			onSessionReady: (payload) => {
				sessionIdRef.current = payload.sessionId
				setSessionId(payload.sessionId)
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

				setMessages((current) => {
					const result = applySessionUpdate(
						current,
						streamingRef.current,
						payload,
					)
					streamingRef.current = result.streamingText
					setStreaming(result.streamingText)
					return result.messages
				})
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
				setMessages((current) => {
					const next = appendStreamToMessages(current, streamingRef.current)
					streamingRef.current = ""
					setStreaming("")
					return next
				})
				setPromptInFlight(false)
				setStatus("idle")
				setPermission(null)
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
	])
}
