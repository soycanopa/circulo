import { useAtom, useSetAtom } from "jotai"
import { useCallback } from "react"
import { GENERAL_CHAT_PROJECT } from "@/lib/preferences"
import {
	archiveSessionId,
	getArchivedSessionIds,
	removeArchivedSessionId,
} from "@/lib/archived-sessions"
import { unpinSessionId } from "@/lib/pinned-sessions"
import { closeSession, createSession, loadSession, openProject } from "@/lib/tauri"
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

	const deleteSession = useCallback(
		async (id: string) => {
			const status = await closeSession(id)
			unpinSessionId(id)
			removeArchivedSessionId(id)
			if (status.activeSessionId === id || status.sessionId === id) {
				resetChatState()
			}
			syncStatus(status)
		},
		[resetChatState, syncStatus],
	)

	const archiveSession = useCallback(
		async (id: string) => {
			archiveSessionId(id)
			unpinSessionId(id)
			if (id !== activeSessionId) return
			const archived = new Set(getArchivedSessionIds())
			const remaining = sessions.filter(
				(session) => !archived.has(session.sessionId),
			)
			const nextId = remaining[0]?.sessionId
			if (!nextId) {
				resetChatState()
				setSessions(remaining)
				setActiveSessionId(null)
				return
			}
			await selectSession(nextId)
		},
		[
			activeSessionId,
			resetChatState,
			selectSession,
			sessions,
			setActiveSessionId,
			setSessions,
		],
	)

	return {
		sessions,
		activeSessionId,
		selectSession,
		newThread,
		newChat,
		deleteSession,
		archiveSession,
		/** @deprecated use newThread */
		newSession: newThread,
	}
}