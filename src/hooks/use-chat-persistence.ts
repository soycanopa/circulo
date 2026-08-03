import { useAtomValue, useSetAtom } from "jotai"
import { useCallback, useEffect, useRef } from "react"
import {
	getDefaultChatsPath,
	listChatSessions,
	persistChatTranscript,
} from "@/lib/tauri"
import { isGeneralChatsPath } from "@/lib/workspace"
import {
	activeSessionIdAtom,
	appSettingsAtom,
	errorMessageAtom,
	generalChatSessionsAtom,
	generalChatsPathAtom,
	projectChatsByPathAtom,
	projectPathAtom,
	visibleMessagesAtom,
	visiblePromptInFlightAtom,
} from "@/stores/atoms"

/** Debounced transcript save — only real ACP session ids (never optimistic). */
export function useChatPersistence() {
	const projectPath = useAtomValue(projectPathAtom)
	const sessionId = useAtomValue(activeSessionIdAtom)
	const messages = useAtomValue(visibleMessagesAtom)
	const promptInFlight = useAtomValue(visiblePromptInFlightAtom)
	const appSettings = useAtomValue(appSettingsAtom)
	const generalChatsPath = useAtomValue(generalChatsPathAtom)

	const setError = useSetAtom(errorMessageAtom)
	const setGeneralChatSessions = useSetAtom(generalChatSessionsAtom)
	const setGeneralChatsPath = useSetAtom(generalChatsPathAtom)
	const setProjectChatsByPath = useSetAtom(projectChatsByPathAtom)
	const lastSavedBySession = useRef<Record<string, string>>({})

	const applySessionsForPath = useCallback(
		(path: string, sessions: Awaited<ReturnType<typeof listChatSessions>>) => {
			if (isGeneralChatsPath(path)) {
				setGeneralChatSessions(sessions)
			} else {
				setProjectChatsByPath((prev) => ({ ...prev, [path]: sessions }))
			}
		},
		[setGeneralChatSessions, setProjectChatsByPath],
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
		setGeneralChatsPath,
	])

	// Initial + when recents / active path change: load full sidebar tree.
	useEffect(() => {
		void refreshAllWorkspaceLists()
	}, [refreshAllWorkspaceLists])

	useEffect(() => {
		if (!projectPath) return
		void refreshPath(projectPath)
	}, [projectPath, refreshPath])

	useEffect(() => {
		if (!projectPath || !sessionId || messages.length === 0) return

		const saveKey = `${projectPath}::${sessionId}`
		const last = messages[messages.length - 1]
		const fingerprint = `${messages.length}:${last?.content.length ?? 0}:${last?.id ?? ""}`
		if (
			fingerprint === lastSavedBySession.current[saveKey] &&
			!promptInFlight
		) {
			return
		}

		const timer = window.setTimeout(() => {
			void persistChatTranscript(projectPath, sessionId, messages)
				.then(() => {
					lastSavedBySession.current[saveKey] = fingerprint
					return refreshPath(projectPath)
				})
				.catch((error: unknown) => {
					const detail = error instanceof Error ? error.message : String(error)
					setError(`Failed to save chat: ${detail}`)
				})
		}, promptInFlight ? 1200 : 600)

		return () => window.clearTimeout(timer)
	}, [
		messages,
		projectPath,
		promptInFlight,
		refreshPath,
		sessionId,
		setError,
	])

	return {
		refreshSessions,
		refreshPath,
		refreshAllWorkspaceLists,
	}
}
