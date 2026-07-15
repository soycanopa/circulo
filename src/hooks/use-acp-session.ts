import { useCallback } from "react"
import { useAtom } from "jotai"
import {
	messagesAtom,
	projectPathAtom,
	promptInFlightAtom,
	replayingHistoryAtom,
	sessionStatusAtom,
	streamingTextAtom,
} from "@/stores/atoms"

/** Read-only view of ACP session state. Event wiring lives in useAcpEventBridge. */
export function useAcpSession() {
	const [messages] = useAtom(messagesAtom)
	const [streamingText] = useAtom(streamingTextAtom)
	const [sessionStatus] = useAtom(sessionStatusAtom)
	const [promptInFlight] = useAtom(promptInFlightAtom)
	const [replayingHistory] = useAtom(replayingHistoryAtom)
	const [projectPath] = useAtom(projectPathAtom)
	const [, setMessages] = useAtom(messagesAtom)
	const [, setStreamingText] = useAtom(streamingTextAtom)

	const resetConversation = useCallback(() => {
		setMessages([])
		setStreamingText("")
	}, [setMessages, setStreamingText])

	return {
		messages,
		streamingText,
		sessionStatus,
		promptInFlight,
		replayingHistory,
		projectPath,
		resetConversation,
	}
}