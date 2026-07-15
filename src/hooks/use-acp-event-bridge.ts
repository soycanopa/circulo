import { useAtomValue, useSetAtom } from "jotai"
import { useEffect, useRef } from "react"
import { applySessionInfoUpdate, applySessionUpdate } from "@/lib/acp-parser"
import { applySessionDefaults } from "@/lib/session-defaults"
import { listenAcpEvents } from "@/lib/tauri"
import {
	activePermissionAtom,
	activeSessionIdAtom,
	agentCapabilitiesAtom,
	configOptionsAtom,
	errorMessageAtom,
	messagesAtom,
	projectPathAtom,
	promptInFlightAtom,
	replayingHistoryAtom,
	sessionStatusAtom,
	sessionsAtom,
	streamingTextAtom,
} from "@/stores/atoms"
import type { ConfigOption, PermissionRequest } from "@/types/acp"

/**
 * Registers ACP event listeners exactly once at the app root.
 * Do not call from multiple components — that duplicates every chunk/tool call.
 */
export function useAcpEventBridge() {
	const setMessages = useSetAtom(messagesAtom)
	const setStreamingText = useSetAtom(streamingTextAtom)
	const setSessionStatus = useSetAtom(sessionStatusAtom)
	const setPromptInFlight = useSetAtom(promptInFlightAtom)
	const setReplayingHistory = useSetAtom(replayingHistoryAtom)
	const setProjectPath = useSetAtom(projectPathAtom)
	const setActiveSessionId = useSetAtom(activeSessionIdAtom)
	const setConfigOptions = useSetAtom(configOptionsAtom)
	const setActivePermission = useSetAtom(activePermissionAtom)
	const setErrorMessage = useSetAtom(errorMessageAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const setCapabilities = useSetAtom(agentCapabilitiesAtom)

	const promptInFlight = useAtomValue(promptInFlightAtom)
	const streamingRef = useRef("")
	const promptInFlightRef = useRef(false)
	const activeSessionRef = useRef<string | null>(null)
	const replayTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

	useEffect(() => {
		promptInFlightRef.current = promptInFlight
	}, [promptInFlight])

	useEffect(() => {
		let unlisteners: Array<() => void> = []

		listenAcpEvents({
			onSessionReady: (payload) => {
				setProjectPath(payload.projectPath)
				setConfigOptions(payload.configOptions)
				setActiveSessionId(payload.sessionId)
				activeSessionRef.current = payload.sessionId
				setSessionStatus("idle")
				setPromptInFlight(false)
				promptInFlightRef.current = false
				setErrorMessage(null)
				setMessages([])
				setStreamingText("")
				streamingRef.current = ""

				void applySessionDefaults(payload.configOptions).catch(() => undefined)

				if (replayTimerRef.current) clearTimeout(replayTimerRef.current)
				replayTimerRef.current = setTimeout(() => {
					setReplayingHistory(false)
					replayTimerRef.current = null
				}, 400)
			},
			onSessionsUpdated: (payload) => {
				setSessions(payload.sessions)
				setActiveSessionId(payload.activeSessionId)
				activeSessionRef.current = payload.activeSessionId
			},
			onSessionUpdate: (payload) => {
				const root = payload as Record<string, unknown>
				const update = root?.update as Record<string, unknown> | undefined
				const updateSessionId = typeof root?.sessionId === "string" ? root.sessionId : null

				if (
					updateSessionId &&
					activeSessionRef.current &&
					updateSessionId !== activeSessionRef.current
				) {
					return
				}

				if (update?.sessionUpdate === "session_info_update") {
					setSessions((current) => applySessionInfoUpdate(current, payload))
					return
				}

				if (update?.sessionUpdate === "config_option_update") {
					const nextOptions = update.configOptions ?? root.configOptions
					if (Array.isArray(nextOptions)) {
						setConfigOptions(nextOptions as ConfigOption[])
					}
					return
				}

				const isStreamingTurn = promptInFlightRef.current

				setMessages((current) => {
					const result = applySessionUpdate(
						current,
						streamingRef.current,
						payload,
						{ streamToMessage: !isStreamingTurn },
					)
					streamingRef.current = result.streamingText
					setStreamingText(result.streamingText)
					return result.messages
				})

				if (promptInFlightRef.current) {
					setSessionStatus("generating")
				}
			},
			onPermissionRequest: (payload) => {
				setSessionStatus("awaiting_permission")
				setActivePermission(payload as PermissionRequest)
			},
			onConfigOptions: (payload) => {
				setConfigOptions(payload.configOptions)
			},
			onPromptComplete: () => {
				setMessages((current) => {
					const stream = streamingRef.current
					if (!stream) return current

					const next = [...current]
					const last = next[next.length - 1]
					if (last?.role === "assistant") {
						last.content = `${last.content}${stream}`
					} else {
						next.push({
							id: crypto.randomUUID(),
							role: "assistant",
							content: stream,
							toolCalls: [],
							timestamp: Date.now(),
						})
					}
					return next
				})

				streamingRef.current = ""
				setStreamingText("")
				setPromptInFlight(false)
				promptInFlightRef.current = false
				setSessionStatus("idle")
			},
			onError: (payload) => {
				setErrorMessage(payload.message)
				streamingRef.current = ""
				setStreamingText("")
				setPromptInFlight(false)
				promptInFlightRef.current = false
				setSessionStatus("idle")
			},
			onDisconnected: () => {
				setSessionStatus("disconnected")
				setPromptInFlight(false)
				promptInFlightRef.current = false
				setReplayingHistory(false)
				setProjectPath(null)
				setSessions([])
				setActiveSessionId(null)
				setCapabilities(null)
				streamingRef.current = ""
				setStreamingText("")
			},
		}).then((listeners) => {
			unlisteners = listeners
		})

		return () => {
			if (replayTimerRef.current) clearTimeout(replayTimerRef.current)
			for (const unlisten of unlisteners) unlisten()
		}
	}, [
		setActivePermission,
		setActiveSessionId,
		setCapabilities,
		setConfigOptions,
		setErrorMessage,
		setMessages,
		setProjectPath,
		setPromptInFlight,
		setReplayingHistory,
		setSessionStatus,
		setSessions,
		setStreamingText,
	])
}