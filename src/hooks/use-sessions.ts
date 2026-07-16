import { useAtom, useSetAtom } from "jotai"
import { useCallback } from "react"
import { getChatsProjectPath } from "@/lib/app-settings"
import {
	archiveSessionId,
	getArchivedSessionIds,
	removeArchivedSessionId,
} from "@/lib/archived-sessions"
import { unpinSessionId } from "@/lib/pinned-sessions"
import { closeSession, createSession, loadSession, openProject, renameSession } from "@/lib/tauri"
import {
	activeSessionIdAtom,
	configOptionsAtom,
	messagesAtom,
	NEW_THREAD_PICKER_ID,
	projectPathAtom,
	replayingHistoryAtom,
	sessionsAtom,
	streamingTextAtom,
	threadFolderPickerSessionIdAtom,
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
	const setThreadFolderPickerSessionId = useSetAtom(threadFolderPickerSessionIdAtom)

	const syncStatus = useCallback(
		(status: Awaited<ReturnType<typeof createSession>>) => {
			setSessions(status.sessions)
			setActiveSessionId(
				status.sessions.length > 0
					? (status.activeSessionId ?? status.sessionId)
					: null,
			)
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
			setThreadFolderPickerSessionId(null)
			setReplayingHistory(true)
			setActiveSessionId(id)
			const status = await loadSession(id)
			syncStatus(status)
		},
		[
			activeSessionId,
			setActiveSessionId,
			setReplayingHistory,
			setThreadFolderPickerSessionId,
			syncStatus,
		],
	)

	const newThread = useCallback(async () => {
		resetChatState()
		setActiveSessionId(null)
		setThreadFolderPickerSessionId(NEW_THREAD_PICKER_ID)
	}, [resetChatState, setActiveSessionId, setThreadFolderPickerSessionId])

	const openProjectForNewThread = useCallback(
		async (path: string) => {
			const status = await openProject(path)
			syncStatus(status)
			const createStatus = await createSession()
			syncStatus(createStatus)
		},
		[syncStatus],
	)

	const newSessionInProject = useCallback(
		async (path: string) => {
			resetChatState()
			setThreadFolderPickerSessionId(null)
			if (projectPath !== path) {
				const status = await openProject(path)
				syncStatus(status)
			}
			const createStatus = await createSession()
			syncStatus(createStatus)
		},
		[projectPath, resetChatState, setThreadFolderPickerSessionId, syncStatus],
	)

	const newChat = useCallback(async () => {
		resetChatState()
		setThreadFolderPickerSessionId(null)
		if (projectPath === getChatsProjectPath()) {
			const status = await createSession()
			syncStatus(status)
			return
		}
		const status = await openProject(getChatsProjectPath())
		syncStatus(status)
	}, [projectPath, resetChatState, setThreadFolderPickerSessionId, syncStatus])

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

	const renameSessionTitle = useCallback(
		async (id: string, title: string) => {
			const status = await renameSession(id, title)
			syncStatus(status)
		},
		[syncStatus],
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
		newSessionInProject,
		openProjectForNewThread,
		deleteSession,
		archiveSession,
		renameSession: renameSessionTitle,
		/** @deprecated use newThread */
		newSession: newThread,
	}
}