import { useAtom, useAtomValue } from "jotai"
import { ChatInput } from "@/components/chat/chat-input"
import { MessageList } from "@/components/chat/message-list"
import { ThreadFolderPicker } from "@/components/chat/thread-folder-picker"
import { PermissionCard } from "@/components/permissions/permission-card"
import { useAcpSession } from "@/hooks/use-acp-session"
import {
	activeSessionIdAtom,
	errorMessageAtom,
	messagesAtom,
	projectPathAtom,
	threadFolderPickerSessionIdAtom,
} from "@/stores/atoms"

interface ChatViewProps {
	connected: boolean
	onOpenProject: (path: string) => Promise<void>
}

export function ChatView({ connected, onOpenProject }: ChatViewProps) {
	const { messages, streamingText, sessionStatus } = useAcpSession()
	const errorMessage = useAtomValue(errorMessageAtom)
	const projectPath = useAtomValue(projectPathAtom)
	const activeSessionId = useAtomValue(activeSessionIdAtom)
	const allMessages = useAtomValue(messagesAtom)
	const [pickerSessionId, setPickerSessionId] = useAtom(threadFolderPickerSessionIdAtom)

	const showFolderPicker =
		Boolean(pickerSessionId) &&
		pickerSessionId === activeSessionId &&
		allMessages.length === 0

	return (
		<div className="relative flex h-full min-h-0 flex-col">
			{showFolderPicker ? (
				<div className="pointer-events-none absolute inset-x-0 top-0 z-20 flex justify-center">
					<div className="-translate-y-1/2 pointer-events-auto pt-px">
						<ThreadFolderPicker
							projectPath={projectPath}
							onOpenProject={onOpenProject}
							onClose={() => setPickerSessionId(null)}
						/>
					</div>
				</div>
			) : null}

			{errorMessage ? (
				<div className="mx-4 mt-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive">
					{errorMessage}
				</div>
			) : null}

			<MessageList
				messages={messages}
				streamingText={streamingText}
				connected={connected}
				empty={!showFolderPicker}
			/>
			<PermissionCard />
			<ChatInput disabled={!connected} sessionStatus={sessionStatus} />
		</div>
	)
}