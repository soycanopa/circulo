import { useAtomValue } from "jotai"
import { useEffect, useRef } from "react"
import { MarkdownContent } from "@/components/chat/markdown-content"
import { PlanPreviewCard } from "@/components/chat/plan-preview-card"
import { ThinkingIndicator } from "@/components/chat/thinking-indicator"
import { useAcpSession } from "@/hooks/use-acp-session"
import { usePlanActions } from "@/hooks/use-plan-actions"
import { isAgentPlanMode } from "@/lib/agent-mode"
import { configOptionsAtom, pendingPlanAtom } from "@/stores/atoms"
import { ToolCallList } from "@/components/tools/tool-call-list"
import type { ChatMessage } from "@/types/acp"

interface MessageListProps {
	messages: ChatMessage[]
	connected: boolean
}

export function MessageList({ messages, connected }: MessageListProps) {
	const { streamingText, promptInFlight } = useAcpSession()
	const configOptions = useAtomValue(configOptionsAtom)
	const pendingPlan = useAtomValue(pendingPlanAtom)
	const { acceptPlan, rejectPlan, startPlanComment, downloadContent } = usePlanActions()

	const bottomRef = useRef<HTMLDivElement>(null)
	const containerRef = useRef<HTMLDivElement>(null)
	const shouldAutoScrollRef = useRef(true)

	const isPlanMode = isAgentPlanMode(configOptions)
	const planContent = streamingText || pendingPlan?.content || ""
	const showPlanPreview =
		isPlanMode &&
		(Boolean(streamingText) || (Boolean(pendingPlan) && !promptInFlight))
	const showThinking = promptInFlight && !streamingText
	const showRegularStream = Boolean(streamingText) && !isPlanMode

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
	}, [messages, streamingText, pendingPlan, promptInFlight, planContent])

	return (
		<div ref={containerRef} className="scrollbar-thin flex-1 overflow-y-auto px-4 py-4">
			{messages.length === 0 && !promptInFlight && !pendingPlan ? (
				<div className="flex h-full items-center justify-center text-sm text-muted-foreground">
					{connected
						? "Envía tu primer mensaje al agente."
						: "Abre un proyecto desde el sidebar para empezar."}
				</div>
			) : null}

			<div className="mx-auto flex max-w-3xl flex-col gap-4">
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

				{showThinking ? <ThinkingIndicator active /> : null}

				{showPlanPreview ? (
					<PlanPreviewCard
						content={planContent}
						isStreaming={promptInFlight}
						actionsEnabled={Boolean(pendingPlan) && !promptInFlight}
						onDownload={() => downloadContent(planContent)}
						onAccept={() => void acceptPlan()}
						onComment={startPlanComment}
						onReject={() => void rejectPlan()}
					/>
				) : null}

				{showRegularStream ? (
					<div className="max-w-full">
						<MarkdownContent content={streamingText} />
					</div>
				) : null}
			</div>

			<div ref={bottomRef} />
		</div>
	)
}