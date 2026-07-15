import { getDefaultStore, useSetAtom } from "jotai"
import { useEffect, useRef } from "react"
import { findModeOption, isAgentPlanMode } from "@/lib/agent-mode"
import { isPlanModeValue } from "@/lib/agent-mode-presentations"
import { normalizePlanMarkdown } from "@/lib/plan-markdown"
import { applySessionInfoUpdate, applySessionUpdate } from "@/lib/acp-parser"
import { promptInFlightRef, setPromptInFlightSync } from "@/lib/prompt-flight"
import { applySessionDefaults } from "@/lib/session-defaults"
import { listenAcpEvents } from "@/lib/tauri"
import {
	activePermissionAtom,
	activeSessionIdAtom,
	agentCapabilitiesAtom,
	configOptionsAtom,
	errorMessageAtom,
	messagesAtom,
	pendingPlanAtom,
	planCommentModeAtom,
	planTurnActiveAtom,
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

	const streamingRef = useRef("")
	const activeSessionRef = useRef<string | null>(null)
	const replayTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

	useEffect(() => {
		let cancelled = false
		let unlisteners: Array<() => void> = []

		listenAcpEvents({
			onSessionReady: (payload) => {
				setProjectPath(payload.projectPath)
				setConfigOptions(payload.configOptions)
				setActiveSessionId(payload.sessionId)
				activeSessionRef.current = payload.sessionId
				setSessionStatus("idle")
				setPromptInFlight(false)
				setPromptInFlightSync(false)
				setErrorMessage(null)
				setMessages([])
				setStreamingText("")
				streamingRef.current = ""
				getDefaultStore().set(pendingPlanAtom, null)
				getDefaultStore().set(planCommentModeAtom, false)

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
				const jotaiStore = getDefaultStore()
				const planTurnActive = jotaiStore.get(planTurnActiveAtom)

				setMessages((current) => {
					const result = applySessionUpdate(
						current,
						streamingRef.current,
						payload,
						{ streamToMessage: !isStreamingTurn && !planTurnActive },
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
				const stream = streamingRef.current
				const jotaiStore = getDefaultStore()
				const configOptions = jotaiStore.get(configOptionsAtom)
				const planTurnActive = jotaiStore.get(planTurnActiveAtom)
				const planMode =
					planTurnActive ||
					isAgentPlanMode(configOptions) ||
					isPlanModeValue(findModeOption(configOptions)?.currentValue)

				if (planMode && stream.trim()) {
					const normalized = normalizePlanMarkdown(stream)
					jotaiStore.set(pendingPlanAtom, {
						content: normalized,
						timestamp: Date.now(),
					})
					jotaiStore.set(planCommentModeAtom, false)
					setMessages((current) => {
						const next = [...current]
						const last = next[next.length - 1]
						if (
							last?.role === "assistant" &&
							(!last.content.trim() ||
								normalized.startsWith(last.content) ||
								last.content.startsWith(normalized))
						) {
							next.pop()
						}
						return next
					})
				} else {
					setMessages((current) => {
						if (!stream) return current

						const next = [...current]
						const last = next[next.length - 1]
						if (last?.role === "assistant") {
							if (stream.startsWith(last.content)) {
								last.content = stream
							} else if (!last.content.endsWith(stream)) {
								last.content = `${last.content}${stream}`
							}
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
				}

				jotaiStore.set(planTurnActiveAtom, false)
				streamingRef.current = ""
				setStreamingText("")
				setPromptInFlight(false)
				setPromptInFlightSync(false)
				setSessionStatus("idle")
			},
			onError: (payload) => {
				setErrorMessage(payload.message)
				streamingRef.current = ""
				setStreamingText("")
				setPromptInFlight(false)
				setPromptInFlightSync(false)
				setSessionStatus("idle")
			},
			onDisconnected: () => {
				setSessionStatus("disconnected")
				setPromptInFlight(false)
				setPromptInFlightSync(false)
				setReplayingHistory(false)
				setProjectPath(null)
				setSessions([])
				setActiveSessionId(null)
				setCapabilities(null)
				streamingRef.current = ""
				setStreamingText("")
			},
		}).then((listeners) => {
			if (cancelled) {
				for (const unlisten of listeners) unlisten()
				return
			}
			unlisteners = listeners
		})

		return () => {
			cancelled = true
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