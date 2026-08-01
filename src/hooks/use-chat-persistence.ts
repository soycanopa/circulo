import { useAtomValue, useSetAtom } from "jotai"
import { useCallback, useEffect, useRef } from "react"
import {
	getDefaultChatsPath,
	listChatSessions,
	persistChatTranscript,
} from "@/lib/tauri"
import { isGeneralChatsPath } from "@/lib/workspace"
import {
	appSettingsAtom,
	chatSessionsAtom,
	generalChatSessionsAtom,
	generalChatsPathAtom,
	messagesAtom,
	projectChatsByPathAtom,
	projectPathAtom,
	promptInFlightAtom,
	sessionIdAtom,
} from "@/stores/atoms"

/** Debounced transcript save — only real ACP session ids (never optimistic). */
export function useChatPersistence() {
	const projectPath = useAtomValue(projectPathAtom)
	const sessionId = useAtomValue(sessionIdAtom)
	const messages = useAtomValue(messagesAtom)
	const promptInFlight = useAtomValue(promptInFlightAtom)
	const appSettings = useAtomValue(appSettingsAtom)
	const generalChatsPath = useAtomValue(generalChatsPathAtom)

	const setChatSessions = useSetAtom(chatSessionsAtom)
	const setGeneralChatSessions = useSetAtom(generalChatSessionsAtom)
	const setGeneralChatsPath = useSetAtom(generalChatsPathAtom)
	const setProjectChatsByPath = useSetAtom(projectChatsByPathAtom)
	const lastSaved = useRef<string>("")

	const applySessionsForPath = useCallback(
		(path: string, sessions: Awaited<ReturnType<typeof listChatSessions>>) => {
			if (isGeneralChatsPath(path)) {
				setGeneralChatSessions(sessions)
			} else {
				setProjectChatsByPath((prev) => ({ ...prev, [path]: sessions }))
			}
			// Keep active-path convenience atom in sync for titles / legacy callers.
			if (projectPath === path) {
				setChatSessions(sessions)
			}
		},
		[
			projectPath,
			setChatSessions,
			setGeneralChatSessions,
			setProjectChatsByPath,
		],
	)

	const refreshPath = useCallback(
		async (path: string) => {
			try {
				const sessions = await listChatSessions(path)
				applySessionsForPath(path, sessions)
				return sessions
			} catch {
				applySessionsForPath(path, [])
				return []
			}
		},
		[applySessionsForPath],
	)

	/** Refresh general Chats + projects of the **active** space. */
	const refreshAllWorkspaceLists = useCallback(async () => {
		let generalPath: string
		try {
			// Always resolve from active workspace (spaces have different chats dirs).
			generalPath = await getDefaultChatsPath()
			setGeneralChatsPath(generalPath)
		} catch {
			return
		}

		// Only projects owned by the active space — never global recentProjects.
		const projectPaths = new Set<string>()
		const activeId = appSettings?.activeWorkspaceId
		const activeWs = appSettings?.workspaces?.find((w) => w.id === activeId)
		for (const path of activeWs?.projectPaths ?? []) {
			if (!isGeneralChatsPath(path)) projectPaths.add(path)
		}

		await Promise.all([
			refreshPath(generalPath),
			...[...projectPaths].map((path) => refreshPath(path)),
		])
	}, [
		appSettings?.activeWorkspaceId,
		appSettings?.workspaces,
		refreshPath,
		setGeneralChatsPath,
	])

	/** Refresh only the active workspace list (plus keep maps coherent). */
	const refreshSessions = useCallback(async () => {
		if (!projectPath) {
			setChatSessions([])
			await refreshAllWorkspaceLists()
			return
		}
		await refreshPath(projectPath)
		// Also refresh general so New chat under projects doesn't leave Chats stale empty.
		if (!isGeneralChatsPath(projectPath)) {
			const generalPath =
				generalChatsPath ??
				(await getDefaultChatsPath().catch(() => null))
			if (generalPath) {
				if (!generalChatsPath) setGeneralChatsPath(generalPath)
				await refreshPath(generalPath)
			}
		}
	}, [
		generalChatsPath,
		projectPath,
		refreshAllWorkspaceLists,
		refreshPath,
		setChatSessions,
		setGeneralChatsPath,
	])

	// Initial + when recents / active path change: load full sidebar tree.
	useEffect(() => {
		void refreshAllWorkspaceLists()
	}, [refreshAllWorkspaceLists])

	// Sync convenience atom when active path changes and we already have cache.
	useEffect(() => {
		if (!projectPath) {
			setChatSessions([])
			return
		}
		void refreshPath(projectPath)
	}, [projectPath, refreshPath, setChatSessions])

	useEffect(() => {
		if (!projectPath || !sessionId || messages.length === 0) return

		const last = messages[messages.length - 1]
		const fingerprint = `${sessionId}:${messages.length}:${last?.content.length ?? 0}`
		if (fingerprint === lastSaved.current && !promptInFlight) return

		const timer = window.setTimeout(() => {
			void persistChatTranscript(projectPath, sessionId, messages)
				.then(() => {
					lastSaved.current = fingerprint
					return refreshPath(projectPath)
				})
				.catch(() => {
					// Non-blocking — chat still works if disk write fails.
				})
		}, promptInFlight ? 1200 : 600)

		return () => window.clearTimeout(timer)
	}, [
		messages,
		projectPath,
		promptInFlight,
		refreshPath,
		sessionId,
	])

	return {
		refreshSessions,
		refreshPath,
		refreshAllWorkspaceLists,
	}
}
