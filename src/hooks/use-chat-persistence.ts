import { useAtomValue, useSetAtom } from "jotai"
import { useCallback, useEffect, useRef } from "react"
import { listChatSessions, persistChatTranscript } from "@/lib/tauri"
import {
	chatSessionsAtom,
	messagesAtom,
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
	const setChatSessions = useSetAtom(chatSessionsAtom)
	const lastSaved = useRef<string>("")

	const refreshSessions = useCallback(async () => {
		if (!projectPath) {
			setChatSessions([])
			return
		}
		const sessions = await listChatSessions(projectPath)
		setChatSessions(sessions)
	}, [projectPath, setChatSessions])

	useEffect(() => {
		void refreshSessions()
	}, [refreshSessions])

	useEffect(() => {
		if (!projectPath || !sessionId || messages.length === 0) return

		const last = messages[messages.length - 1]
		const fingerprint = `${sessionId}:${messages.length}:${last?.content.length ?? 0}`
		if (fingerprint === lastSaved.current && !promptInFlight) return

		const timer = window.setTimeout(() => {
			void persistChatTranscript(projectPath, sessionId, messages)
				.then(() => {
					lastSaved.current = fingerprint
					return refreshSessions()
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
		refreshSessions,
		sessionId,
	])

	return { refreshSessions }
}
