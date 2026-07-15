import { useAtom, useSetAtom } from "jotai"
import { useCallback } from "react"
import { GENERAL_CHAT_PROJECT } from "@/lib/preferences"
import { createSession, loadSession, openProject } from "@/lib/tauri"
import {
	activeSessionIdAtom,
	configOptionsAtom,
	messagesAtom,
	projectPathAtom,
	replayingHistoryAtom,
	sessionsAtom,
	streamingTextAtom,
} from "@/stores/atoms"

export function useSessions() {
	const [sessions, setSessions] = useAtom(sessionsAtom)
	const [activeSessionId, setActiveSessionId] = useAtom(activeSessionIdAtom)
	const [projectPath] = useAtom(projectPathAtom)
	const setMessages = useSetAtom(messagesAtom)
	const setStreamingText = useSetAtom(streamingTextAtom)
	const setConfigOptions = useSetAtom(configOptionsAtom)
	const setProjectPath = useSetAtom(projectPathAtom)
	const setReplayingHistory = useSetAtom(replayingHistoryAtom)

	const syncStatus = useCallback(
		(status: Awaited<ReturnType<typeof createSession>>) => {
			setSessions(status.sessions)
			setActiveSessionId(status.activeSessionId ?? status.sessionId)
			setProjectPath(status.projectPath)
		},
		[setSessions, setActiveSessionId, setProjectPath],
	)

	const resetChatState = useCallback(() => {
		setMessages([])
		setStreamingText("")
		setConfigOptions([])
		setReplayingHistory(false)
	}, [setMessages, setStreamingText, setConfigOptions, setReplayingHistory])

	const selectSession = useCallback(
		async (id: string) => {
			if (id === activeSessionId) return
			setReplayingHistory(true)
			setMessages([])
			setStreamingText("")
			const status = await loadSession(id)
			syncStatus(status)
		},
		[activeSessionId, setMessages, setStreamingText, setReplayingHistory, syncStatus],
	)

	const newThread = useCallback(async () => {
		resetChatState()
		const status = await createSession()
		syncStatus(status)
	}, [resetChatState, syncStatus])

	const newChat = useCallback(async () => {
		resetChatState()
		if (projectPath === GENERAL_CHAT_PROJECT) {
			const status = await createSession()
			syncStatus(status)
			return
		}
		const status = await openProject(GENERAL_CHAT_PROJECT)
		syncStatus(status)
	}, [projectPath, resetChatState, syncStatus])

	return {
		sessions,
		activeSessionId,
		selectSession,
		newThread,
		newChat,
		/** @deprecated use newThread */
		newSession: newThread,
	}
}