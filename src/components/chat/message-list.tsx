import { useEffect, useRef } from "react"
import { MarkdownContent } from "@/components/chat/markdown-content"
import { ToolCallList } from "@/components/tools/tool-call-list"
import type { ChatMessage } from "@/types/acp"

interface MessageListProps {
	messages: ChatMessage[]
	streamingText: string
	connected: boolean
}

export function MessageList({ messages, streamingText, connected }: MessageListProps) {
	const bottomRef = useRef<HTMLDivElement>(null)
	const containerRef = useRef<HTMLDivElement>(null)
	const shouldAutoScrollRef = useRef(true)

	useEffect(() => {
		const container = containerRef.current
		if (!container) return
		function handleScroll() {
			const distanceFromBottom =
				container!.scrollHeight - container!.scrollTop - container!.clientHeight
			shouldAutoScrollRef.current = distanceFromBottom < 80
		}
		container.addEventListener("scroll", handleScroll)
		return () => container.removeEventListener("scroll", handleScroll)
	}, [])

	useEffect(() => {
		if (!shouldAutoScrollRef.current) return
		bottomRef.current?.scrollIntoView({ behavior: "smooth" })
	}, [messages, streamingText])

	return (
		<div ref={containerRef} className="scrollbar-thin flex-1 overflow-y-auto px-4 py-4">
			{messages.length === 0 ? (
				<div className="flex h-full items-center justify-center text-sm text-muted-foreground">
					{connected
						? "Envía tu primer mensaje al agente."
						: "Abre un proyecto desde el sidebar para empezar."}
				</div>
			) : null}

			<div className="mx-auto flex max-w-4xl flex-col gap-4">
				{messages.map((message) => (
					<div
						key={message.id}
						className={
							message.role === "user"
								? "ml-auto max-w-[85%] rounded-xl bg-secondary px-4 py-3 text-sm"
								: "max-w-full"
						}
					>
						{message.role === "assistant" ? (
							<>
								<ToolCallList toolCalls={message.toolCalls} />
								{message.content ? <MarkdownContent content={message.content} /> : null}
							</>
						) : (
							<p className="whitespace-pre-wrap">{message.content}</p>
						)}
					</div>
				))}

				{streamingText ? (
					<div className="max-w-full">
						<MarkdownContent content={streamingText} />
						<span className="loading-shimmer text-xs text-muted-foreground">Generando…</span>
					</div>
				) : null}
			</div>

			<div ref={bottomRef} />
		</div>
	)
}
