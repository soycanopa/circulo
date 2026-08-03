import { useAtomValue, useSetAtom } from "jotai"
import { useEffect, useRef } from "react"
import { ToolCallCard } from "@/components/tools/tool-call-card"
import { SimpleMarkdown } from "@/lib/simple-markdown"
import type { ToolCall } from "@/types/acp"
import {
	diffPanelOpenAtom,
	selectedDiffToolAtom,
	visibleMessagesAtom,
	visiblePromptInFlightAtom,
	visibleStreamingAtom,
} from "@/stores/atoms"

export function MessageList() {
	const messages = useAtomValue(visibleMessagesAtom)
	const streaming = useAtomValue(visibleStreamingAtom)
	const inFlight = useAtomValue(visiblePromptInFlightAtom)
	const setSelectedDiff = useSetAtom(selectedDiffToolAtom)
	const setDiffPanelOpen = useSetAtom(diffPanelOpenAtom)

	function openDiffPanel(tool: ToolCall) {
		setSelectedDiff(tool)
		setDiffPanelOpen(true)
	}
	const scrollRef = useRef<HTMLDivElement>(null)
	const bottomRef = useRef<HTMLDivElement>(null)

	useEffect(() => {
		bottomRef.current?.scrollIntoView({ block: "end", behavior: "smooth" })
	}, [messages, streaming, inFlight])

	if (messages.length === 0 && !streaming) {
		return (
			<div className="flex flex-1 items-center justify-center px-6 text-center text-sm text-muted">
				Send a message to start the conversation.
			</div>
		)
	}

	const last = messages[messages.length - 1]
	const showCaret =
		inFlight &&
		last?.role === "assistant" &&
		!last.content &&
		last.toolCalls.length === 0 &&
		!streaming

	return (
		<div
			ref={scrollRef}
			className="flex-1 space-y-4 overflow-y-auto px-4 py-4"
		>
			{messages.map((message) => {
				const isLiveAssistant =
					inFlight &&
					message.role === "assistant" &&
					message.id === last?.id
				return (
					<div key={message.id} className="mx-auto max-w-3xl">
						<div className="mb-1 text-[11px] uppercase tracking-wide text-muted">
							{message.role === "user" ? "You" : "Agent"}
						</div>
						{message.content ? (
							<div className="text-sm leading-relaxed text-fg">
								{message.role === "assistant" ? (
									<SimpleMarkdown text={message.content} />
								) : (
									<p className="whitespace-pre-wrap">{message.content}</p>
								)}
								{isLiveAssistant ? (
									<span className="ml-0.5 inline-block h-3.5 w-1.5 translate-y-0.5 animate-pulse bg-fg/70" />
								) : null}
							</div>
						) : message.role === "assistant" &&
						  showCaret &&
						  message.id === last?.id ? (
							<div className="flex items-center gap-2 text-sm text-muted">
								<span className="inline-block h-3.5 w-1.5 animate-pulse bg-fg/70" />
								<span>Thinking…</span>
							</div>
						) : null}
						{message.toolCalls.length > 0 ? (
							<div className="mt-2 space-y-1.5">
								{message.toolCalls.map((tool) => (
									<ToolCallCard
										key={tool.id}
										tool={tool}
										onOpenDiff={openDiffPanel}
									/>
								))}
							</div>
						) : null}
					</div>
				)
			})}
			{streaming ? (
				<div className="mx-auto max-w-3xl">
					<div className="mb-1 text-[11px] uppercase tracking-wide text-muted">
						Agent
					</div>
					<div className="text-sm leading-relaxed text-fg">
						<SimpleMarkdown text={streaming} />
						<span className="ml-0.5 inline-block h-3.5 w-1.5 translate-y-0.5 animate-pulse bg-fg/70" />
					</div>
				</div>
			) : null}
			<div ref={bottomRef} aria-hidden className="h-px shrink-0" />
		</div>
	)
}
