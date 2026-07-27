import { useAtomValue } from "jotai"
import {
	messagesAtom,
	promptInFlightAtom,
	streamingTextAtom,
} from "@/stores/atoms"
import { ToolCallCard } from "@/components/tools/tool-call-card"

export function MessageList() {
	const messages = useAtomValue(messagesAtom)
	const streaming = useAtomValue(streamingTextAtom)
	const inFlight = useAtomValue(promptInFlightAtom)

	if (messages.length === 0 && !streaming) {
		return (
			<div className="flex flex-1 items-center justify-center px-6 text-center text-sm text-muted">
				Send a message to start the conversation.
			</div>
		)
	}

	return (
		<div className="flex-1 space-y-4 overflow-y-auto px-4 py-4">
			{messages.map((message) => (
				<div key={message.id} className="mx-auto max-w-3xl">
					<div className="mb-1 text-[11px] uppercase tracking-wide text-muted">
						{message.role === "user" ? "You" : "Agent"}
					</div>
					{message.content ? (
						<div className="whitespace-pre-wrap text-sm leading-relaxed text-fg">
							{message.content}
						</div>
					) : null}
					{message.toolCalls.length > 0 ? (
						<div className="mt-2 space-y-1.5">
							{message.toolCalls.map((tool) => (
								<ToolCallCard key={tool.id} tool={tool} />
							))}
						</div>
					) : null}
				</div>
			))}
			{(streaming || inFlight) && (
				<div className="mx-auto max-w-3xl">
					<div className="mb-1 text-[11px] uppercase tracking-wide text-muted">
						Agent
					</div>
					<div className="whitespace-pre-wrap text-sm leading-relaxed text-fg">
						{streaming || (inFlight ? "…" : "")}
					</div>
				</div>
			)}
		</div>
	)
}
