import { getDefaultStore } from "jotai"
import { useAtom, useSetAtom } from "jotai"
import { useCallback, useRef } from "react"
import { setLastProjectPath } from "@/lib/app-bootstrap"
import { getAppSettings, getChatsProjectPath } from "@/lib/app-settings"
import { isGeneralChatProject } from "@/lib/project-display"
import {
	archiveSessionId,
	getArchivedSessionIds,
	removeArchivedSessionId,
} from "@/lib/archived-sessions"
import { unpinSessionId } from "@/lib/pinned-sessions"
import {
	createOptimisticSessionEntry,
	isOptimisticSessionId,
	OPTIMISTIC_SESSION_ID,
} from "@/lib/optimistic-session"
import { normalizeSessionId } from "@/lib/session-id"
import { addRecentProject } from "@/lib/recent-projects"
import { flushPendingPrompt } from "@/lib/pending-prompt"
import { waitForAgentReady } from "@/lib/wait-for-agent-ready"
import {
	closeProject,
	closeSession,
	createSession,
	loadSession,
	openProject,
	renameSession,
	type CloseSessionResult,
} from "@/lib/tauri"
import type { SessionInfo } from "@/types/acp"
import {
	activeSessionIdAtom,
	configOptionsAtom,
	creatingSessionAtom,
	errorMessageAtom,
	messagesAtom,
	NEW_THREAD_PICKER_ID,
	pendingPromptAtom,
	projectPathAtom,
	replayingHistoryAtom,
	sessionsAtom,
	streamingTextAtom,
	threadFolderPickerSessionIdAtom,
} from "@/stores/atoms"

function removeOptimisticSession(
	setSessions: (update: (current: SessionInfo[]) => SessionInfo[]) => void,
	setActiveSessionId: (value: string | null | ((current: string | null) => string | null)) => void,
) {
	setSessions((current) => current.filter((session) => !isOptimisticSessionId(session.sessionId)))
	setActiveSessionId((current) => (isOptimisticSessionId(current) ? null : current))
}

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
	const setCreatingSession = useSetAtom(creatingSessionAtom)
	const createGenerationRef = useRef(0)

	const syncStatus = useCallback(
		(status: Awaited<ReturnType<typeof createSession>>) => {
			setSessions(status.sessions)
			const nextSessionId = normalizeSessionId(
				status.sessions.length > 0
					? (status.activeSessionId ?? status.sessionId)
					: null,
			)
			if (nextSessionId) setActiveSessionId(nextSessionId)
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

	const beginOptimisticSession = useCallback(
		(targetPath: string) => {
			const generation = ++createGenerationRef.current
			resetChatState()
			setThreadFolderPickerSessionId(null)
			setSessions((current) => [
				createOptimisticSessionEntry(targetPath),
				...current.filter((session) => !isOptimisticSessionId(session.sessionId)),
			])
			setActiveSessionId(OPTIMISTIC_SESSION_ID)
			setProjectPath(targetPath)
			setCreatingSession(true)
			return generation
		},
		[
			resetChatState,
			setActiveSessionId,
			setCreatingSession,
			setProjectPath,
			setSessions,
			setThreadFolderPickerSessionId,
		],
	)

	const finishOptimisticCreate = useCallback(
		(
			generation: number,
			run: () => Promise<void>,
		) => {
			void (async () => {
				try {
					await run()
					if (generation !== createGenerationRef.current) return
				} catch (error) {
					if (generation !== createGenerationRef.current) return
					removeOptimisticSession(setSessions, setActiveSessionId)
					getDefaultStore().set(pendingPromptAtom, null)
					getDefaultStore().set(
						errorMessageAtom,
						error instanceof Error ? error.message : "No se pudo crear la sesión",
					)
				} finally {
					if (generation === createGenerationRef.current) {
						setCreatingSession(false)
					}
				}
			})()
		},
		[setActiveSessionId, setCreatingSession, setSessions],
	)

	const selectSession = useCallback(
		async (id: string) => {
			if (isOptimisticSessionId(id) || id === activeSessionId) return

			if (isOptimisticSessionId(activeSessionId)) {
				createGenerationRef.current += 1
				setCreatingSession(false)
				getDefaultStore().set(pendingPromptAtom, null)
				removeOptimisticSession(setSessions, setActiveSessionId)
			}

			setThreadFolderPickerSessionId(null)
			resetChatState()
			setReplayingHistory(true)
			setActiveSessionId(id)
			const status = await loadSession(id)
			syncStatus(status)
		},
		[
			activeSessionId,
			setActiveSessionId,
			setCreatingSession,
			setReplayingHistory,
			setSessions,
			setThreadFolderPickerSessionId,
			syncStatus,
			resetChatState,
		],
	)

	const newThread = useCallback(async () => {
		resetChatState()
		setActiveSessionId(null)
		setThreadFolderPickerSessionId(NEW_THREAD_PICKER_ID)
	}, [resetChatState, setActiveSessionId, setThreadFolderPickerSessionId])

	const openProjectForNewThread = useCallback(
		(path: string) => {
			if (
				projectPath &&
				!isGeneralChatProject(projectPath) &&
				projectPath !== path
			) {
				addRecentProject(projectPath)
			}
			if (!isGeneralChatProject(path)) {
				setLastProjectPath(path)
				addRecentProject(path)
			}

			const generation = beginOptimisticSession(path)
			const needsOpen = projectPath !== path

			finishOptimisticCreate(generation, async () => {
				if (needsOpen) {
					await openProject(path, {
						agentId: getAppSettings().defaultProvider,
						deferSessionBootstrap: true,
					})
					await waitForAgentReady()
				}
				const status = await createSession()
				syncStatus(status)
				await flushPendingPrompt()
			})
		},
		[beginOptimisticSession, finishOptimisticCreate, projectPath, syncStatus],
	)

	const newSessionInProject = useCallback(
		(path: string) => {
			if (projectPath !== path) {
				if (projectPath && !isGeneralChatProject(projectPath)) {
					addRecentProject(projectPath)
				}
				if (!isGeneralChatProject(path)) {
					setLastProjectPath(path)
					addRecentProject(path)
				}
			}

			const generation = beginOptimisticSession(path)
			const needsOpen = projectPath !== path

			finishOptimisticCreate(generation, async () => {
				if (needsOpen) {
					await openProject(path, {
						agentId: getAppSettings().defaultProvider,
						deferSessionBootstrap: true,
					})
					await waitForAgentReady()
				}
				const status = await createSession()
				syncStatus(status)
				await flushPendingPrompt()
			})
		},
		[beginOptimisticSession, finishOptimisticCreate, projectPath, syncStatus],
	)

	const newChat = useCallback(() => {
		const chatsPath = getChatsProjectPath()
		const generation = beginOptimisticSession(chatsPath)
		const needsOpen = projectPath !== chatsPath

		finishOptimisticCreate(generation, async () => {
			setLastProjectPath(null)
			if (needsOpen) {
				await openProject(chatsPath, {
					agentId: getAppSettings().defaultProvider,
					deferSessionBootstrap: true,
				})
				await waitForAgentReady()
			}
			const status = await createSession()
			syncStatus(status)
			await flushPendingPrompt()
		})
	}, [beginOptimisticSession, finishOptimisticCreate, projectPath, syncStatus])

	const deleteSession = useCallback(
		async (sessionProjectPath: string, id: string) => {
			if (isOptimisticSessionId(id)) {
				createGenerationRef.current += 1
				const remaining = sessions.filter((s) => !isOptimisticSessionId(s.sessionId))
				removeOptimisticSession(setSessions, setActiveSessionId)
				resetChatState()
				setCreatingSession(false)
				getDefaultStore().set(pendingPromptAtom, null)
				return {
					status: {
						connected: false,
						projectPath: sessionProjectPath,
						agentId: null,
						sessionId: null,
						activeSessionId: null,
						agentCommand: "",
						sessions: remaining,
						capabilities: null,
					},
					projectPath: sessionProjectPath,
					sessions: remaining,
				} satisfies CloseSessionResult
			}

			const result = await closeSession(id, sessionProjectPath)
			unpinSessionId(id)
			removeArchivedSessionId(id)

			const { status } = result
			const clearedActive =
				normalizeSessionId(status.activeSessionId ?? status.sessionId) === null &&
				(activeSessionId === id || normalizeSessionId(status.sessionId) === id)

			if (status.activeSessionId === id || status.sessionId === id || clearedActive) {
				resetChatState()
				setActiveSessionId(null)
			}

			if (projectPath === sessionProjectPath) {
				syncStatus(status)
			}

			if (
				projectPath === sessionProjectPath &&
				status.sessions.length === 0 &&
				isGeneralChatProject(status.projectPath)
			) {
				const closedStatus = await closeProject()
				syncStatus(closedStatus)
			}

			return result
		},
		[
			activeSessionId,
			projectPath,
			resetChatState,
			setActiveSessionId,
			setCreatingSession,
			setSessions,
			sessions,
			syncStatus,
		],
	)

	const renameSessionTitle = useCallback(
		async (id: string, title: string) => {
			if (isOptimisticSessionId(id)) {
				setSessions((current) =>
					current.map((session) =>
						session.sessionId === id ? { ...session, title } : session,
					),
				)
				return
			}
			const status = await renameSession(id, title)
			syncStatus(status)
		},
		[setSessions, syncStatus],
	)

	const archiveSession = useCallback(
		async (id: string) => {
			if (isOptimisticSessionId(id)) return
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