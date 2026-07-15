import { useAtomValue } from "jotai"
import { ChatInput } from "@/components/chat/chat-input"
import { MessageList } from "@/components/chat/message-list"
import { PermissionCard } from "@/components/permissions/permission-card"
import { useAcpSession } from "@/hooks/use-acp-session"
import { errorMessageAtom } from "@/stores/atoms"

interface ChatViewProps {
	connected: boolean
	onOpenProject: (path: string) => Promise<void>
}

export function ChatView({ connected, onOpenProject }: ChatViewProps) {
	const { messages, streamingText, sessionStatus } = useAcpSession()
	const errorMessage = useAtomValue(errorMessageAtom)

	return (
		<div className="flex h-full min-h-0 flex-col">
			{errorMessage ? (
				<div className="mx-4 mt-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive">
					{errorMessage}
				</div>
			) : null}

			<MessageList messages={messages} streamingText={streamingText} connected={connected} />
			<PermissionCard />
			<ChatInput
				disabled={!connected}
				sessionStatus={sessionStatus}
				onOpenProject={onOpenProject}
			/>
		</div>
	)
}