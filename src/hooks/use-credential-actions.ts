import { useCallback } from "react"
import { summarizeCredentialSubmission } from "@/lib/credential-presentation"
import type {
	ChatMessage,
	CredentialRequest,
	CredentialResponseAction,
} from "@/types/acp"

export function useCredentialHistory() {
	const recordCredentialTurn = useCallback(
		(
			setMessages: (updater: (current: ChatMessage[]) => ChatMessage[]) => void,
			request: CredentialRequest,
			values: Record<string, string>,
			action: CredentialResponseAction,
		) => {
			const status =
				action === "accept" ? "provided" : action === "decline" ? "declined" : "cancelled"

			setMessages((current) => [
				...current,
				{
					id: crypto.randomUUID(),
					role: "assistant",
					kind: "auth-request",
					content: summarizeCredentialSubmission(request, values, action),
					authMeta: {
						title: request.title,
						mode: request.mode,
						status,
					},
					toolCalls: [],
					timestamp: Date.now(),
				},
			])
		},
		[],
	)

	return { recordCredentialTurn }
}