import { getDefaultStore, useSetAtom } from "jotai"
import { useEffect, useRef } from "react"
import { isAgentPlanMode } from "@/lib/agent-mode"
import { isPlanLikeContent, normalizePlanMarkdown } from "@/lib/plan-markdown"
import { applySessionInfoUpdate, applySessionUpdate } from "@/lib/acp-parser"
import { parseUsageUpdate } from "@/lib/context-window"
import { promptInFlightRef, setPromptInFlightSync } from "@/lib/prompt-flight"
import { AGENT_READY_EVENT } from "@/lib/wait-for-agent-ready"
import { applySessionDefaults } from "@/lib/session-defaults"
import { isOptimisticSessionId } from "@/lib/optimistic-session"
import { flushPendingPrompt } from "@/lib/pending-prompt"
import {
	cacheSessionMessages,
	getCachedSessionMessages,
} from "@/lib/session-messages-cache"
import { normalizeSessionId } from "@/lib/session-id"
import { listenAcpEvents } from "@/lib/tauri"
import {
	activeCredentialAtom,
	activePermissionAtom,
	activeSessionIdAtom,
	agentConnectedAtom,
	agentCapabilitiesAtom,
	configOptionsAtom,
	contextWindowAtom,
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
import { normalizeCredentialRequest } from "@/lib/credential-presentation"
import type { ConfigOption, PermissionRequest } from "@/types/acp"

const HISTORY_REPLAY_MS = 2500

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
	const setActiveCredential = useSetAtom(activeCredentialAtom)
	const setErrorMessage = useSetAtom(errorMessageAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const setAgentConnected = useSetAtom(agentConnectedAtom)
	const setCapabilities = useSetAtom(agentCapabilitiesAtom)

	const streamingRef = useRef("")
	const activeSessionRef = useRef<string | null>(null)
	const replayTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

	const scheduleReplayEnd = (setReplaying: (value: boolean) => void) => {
		if (replayTimerRef.current) clearTimeout(replayTimerRef.current)
		replayTimerRef.current = setTimeout(() => {
			setReplaying(false)
			replayTimerRef.current = null
		}, HISTORY_REPLAY_MS)
	}

	const resolveAcceptedSessionId = (): string | null => {
		const fromAtom = getDefaultStore().get(activeSessionIdAtom)
		const candidate = normalizeSessionId(fromAtom ?? activeSessionRef.current)
		return candidate
	}

	useEffect(() => {
		let cancelled = false
		let unlisteners: Array<() => void> = []

		listenAcpEvents({
			onAgentReady: (payload) => {
				setAgentConnected(true)
				setProjectPath(payload.projectPath)
				setCapabilities(payload.capabilities)
				setSessionStatus("idle")
				setErrorMessage(null)
				window.dispatchEvent(new CustomEvent(AGENT_READY_EVENT))
			},
			onSessionReady: (payload) => {
				const store = getDefaultStore()
				const activeFromAtom = store.get(activeSessionIdAtom)
				const wasOptimistic =
					isOptimisticSessionId(activeFromAtom) ||
					isOptimisticSessionId(activeSessionRef.current)
				const previousSessionId = wasOptimistic
					? null
					: normalizeSessionId(activeSessionRef.current)
				const nextSessionId = payload.sessionId
				const sessionChanged =
					!wasOptimistic &&
					previousSessionId !== null &&
					previousSessionId !== nextSessionId

				if (wasOptimistic) {
					setSessions((current) =>
						current.map((session) =>
							isOptimisticSessionId(session.sessionId)
								? { ...session, sessionId: nextSessionId }
								: session,
						),
					)
				}

				if (sessionChanged && previousSessionId) {
					cacheSessionMessages(previousSessionId, store.get(messagesAtom))
				}

				setProjectPath(payload.projectPath)
				setConfigOptions(payload.configOptions)
				setActiveSessionId(nextSessionId)
				activeSessionRef.current = nextSessionId
				setSessionStatus("idle")
				setErrorMessage(null)

				if (sessionChanged) {
					const cachedMessages = getCachedSessionMessages(nextSessionId)
					if (cachedMessages) {
						setMessages(cachedMessages)
						setStreamingText("")
						streamingRef.current = ""
					} else {
						setMessages([])
						setStreamingText("")
						streamingRef.current = ""
					}
					setPromptInFlight(false)
					setPromptInFlightSync(false)
				}

				store.set(pendingPlanAtom, null)
				store.set(activeCredentialAtom, null)
				store.set(planCommentModeAtom, false)

				void applySessionDefaults(payload.configOptions).catch(() => undefined)

				if (!wasOptimistic) {
					setReplayingHistory(true)
					scheduleReplayEnd(setReplayingHistory)
				}

				void flushPendingPrompt()
			},
			onSessionsUpdated: (payload) => {
				const store = getDefaultStore()
				const hasOptimistic = store
					.get(sessionsAtom)
					.some((session) => isOptimisticSessionId(session.sessionId))

				if (hasOptimistic) {
					setSessions((current) => {
						const optimistic = current.find((session) =>
							isOptimisticSessionId(session.sessionId),
						)
						const merged = payload.sessions.filter(
							(session) => !isOptimisticSessionId(session.sessionId),
						)
						return optimistic ? [optimistic, ...merged] : payload.sessions
					})
				} else {
					setSessions(payload.sessions)
				}

				const nextId = normalizeSessionId(payload.activeSessionId)
				if (nextId) {
					setActiveSessionId(nextId)
					activeSessionRef.current = nextId
				}
			},
			onSessionUpdate: (payload) => {
				const root = payload as Record<string, unknown>
				const update = root?.update as Record<string, unknown> | undefined
				const updateSessionId =
					typeof root?.sessionId === "string" ? root.sessionId : null

				const acceptedSessionId = resolveAcceptedSessionId()
				if (
					updateSessionId &&
					acceptedSessionId &&
					updateSessionId !== acceptedSessionId
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

				const usageSnapshot = parseUsageUpdate(payload)
				if (usageSnapshot) {
					getDefaultStore().set(contextWindowAtom, usageSnapshot)
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

					if (result.messages.length > 0 && jotaiStore.get(replayingHistoryAtom)) {
						scheduleReplayEnd(setReplayingHistory)
					}

					const sessionId = resolveAcceptedSessionId()
					if (sessionId) {
						cacheSessionMessages(sessionId, result.messages)
					}

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
			onCredentialRequest: (payload) => {
				const request = normalizeCredentialRequest(payload)
				if (!request) return
				setSessionStatus("awaiting_credential")
				setActiveCredential(request)
			},
			onConfigOptions: (payload) => {
				setConfigOptions(payload.configOptions)
			},
			onPromptComplete: () => {
				const stream = streamingRef.current
				const jotaiStore = getDefaultStore()
				const configOptions = jotaiStore.get(configOptionsAtom)
				const planTurnActive = jotaiStore.get(planTurnActiveAtom)
				const expectsPlan = planTurnActive || isAgentPlanMode(configOptions)
				const shouldCapturePlan = expectsPlan && stream.trim() && isPlanLikeContent(stream)

				if (shouldCapturePlan) {
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
						const sessionId = resolveAcceptedSessionId()
						if (sessionId) cacheSessionMessages(sessionId, next)
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
						const sessionId = resolveAcceptedSessionId()
						if (sessionId) cacheSessionMessages(sessionId, next)
						return next
					})
				}

				jotaiStore.set(planTurnActiveAtom, false)
				streamingRef.current = ""
				setStreamingText("")
				setPromptInFlight(false)
				setPromptInFlightSync(false)
				setSessionStatus("idle")
				setReplayingHistory(false)
			},
			onError: (payload) => {
				setErrorMessage(payload.message)
				streamingRef.current = ""
				setStreamingText("")
				setPromptInFlight(false)
				setPromptInFlightSync(false)
				setSessionStatus("idle")
				setReplayingHistory(false)
			},
			onDisconnected: () => {
				setAgentConnected(false)
				setSessionStatus("disconnected")
				setPromptInFlight(false)
				setPromptInFlightSync(false)
				setReplayingHistory(false)
				setProjectPath(null)
				setSessions([])
				setActiveSessionId(null)
				activeSessionRef.current = null
				setCapabilities(null)
				streamingRef.current = ""
				setStreamingText("")
				getDefaultStore().set(contextWindowAtom, null)
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
		setActiveCredential,
		setActivePermission,
		setActiveSessionId,
		setAgentConnected,
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