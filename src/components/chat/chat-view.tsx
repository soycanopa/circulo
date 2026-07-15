import { useAtomValue } from "jotai"
import { ChatInput } from "@/components/chat/chat-input"
import { MessageList } from "@/components/chat/message-list"
import { ModelSelector } from "@/components/chat/model-selector"
import { PermissionCard } from "@/components/permissions/permission-card"
import { useAcpSession } from "@/hooks/use-acp-session"
import { errorMessageAtom } from "@/stores/atoms"

interface ChatViewProps {
	connected: boolean
}

export function ChatView({ connected }: ChatViewProps) {
	const { messages, streamingText, sessionStatus } = useAcpSession()
	const errorMessage = useAtomValue(errorMessageAtom)

	return (
		<div className="flex h-full min-h-0 flex-col">
			<div className="flex items-center justify-between border-b border-border px-4 py-3">
				<div>
					<h2 className="text-sm font-medium">Sesión del agente</h2>
					<p className="text-xs text-muted-foreground">
						{connected ? `Estado: ${sessionStatus}` : "Sin proyecto abierto"}
					</p>
				</div>
				<ModelSelector />
			</div>

			{errorMessage ? (
				<div className="mx-4 mt-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive">
					{errorMessage}
				</div>
			) : null}

			<MessageList messages={messages} streamingText={streamingText} />
			<PermissionCard />
			<ChatInput disabled={!connected} sessionStatus={sessionStatus} />
		</div>
	)
}