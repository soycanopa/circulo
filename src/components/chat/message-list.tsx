import { useMemo, useRef } from "react"
import { useAtomValue } from "jotai"
import { MarkdownContent } from "@/components/chat/markdown-content"
import { MessageTrail } from "@/components/chat/message-trail"
import { PlanPreviewCard } from "@/components/chat/plan-preview-card"
import { ThinkingIndicator } from "@/components/chat/thinking-indicator"
import { useAcpSession } from "@/hooks/use-acp-session"
import { useMessageTrail } from "@/hooks/use-message-trail"
import { usePlanActions } from "@/hooks/use-plan-actions"
import { isPlanLikeContent } from "@/lib/plan-markdown"
import { deriveTurnPhase, shouldShowThinkingIndicator } from "@/lib/turn-phase"
import { cn } from "@/lib/utils"
import { pendingPlanAtom, planTurnActiveAtom } from "@/stores/atoms"
import { AuthRequestCard } from "@/components/credentials/auth-request-card"
import { ActivityTrace } from "@/components/tools/activity-trace"
import { ToolCallList } from "@/components/tools/tool-call-list"
import type { ChatMessage } from "@/types/acp"

interface MessageListProps {
	messages: ChatMessage[]
	connected: boolean
}

export function MessageList({ messages, connected }: MessageListProps) {
	const { streamingText, promptInFlight, sessionStatus, replayingHistory } =
		useAcpSession()
	const lastRenderedMessagesRef = useRef(messages)
	if (messages.length > 0) {
		lastRenderedMessagesRef.current = messages
	}

	const trailMessages =
		replayingHistory && messages.length === 0
			? lastRenderedMessagesRef.current
			: messages
	const { scrollRef, activeStore, trailItems, scrollToMessage } =
		useMessageTrail(trailMessages)
	const pendingPlan = useAtomValue(pendingPlanAtom)
	const planTurnActive = useAtomValue(planTurnActiveAtom)
	const { acceptPlan, acceptAndCompactPlan, rejectPlan, startPlanComment, downloadContent } =
		usePlanActions()

	const planLikeStream = isPlanLikeContent(streamingText)
	const showPlanPreview =
		Boolean(pendingPlan) ||
		(planTurnActive && Boolean(streamingText.trim()) && planLikeStream)
	const planContent =
		pendingPlan?.content || (planLikeStream ? streamingText : "") || ""

	const liveTurnTools = useMemo(() => {
		if (!promptInFlight) return []
		const last = messages[messages.length - 1]
		return last?.role === "assistant" ? last.toolCalls : []
	}, [messages, promptInFlight])

	const turnPhase = deriveTurnPhase({
		promptInFlight,
		sessionStatus,
		streamingText,
		toolCalls: liveTurnTools,
	})

	const showThinking =
		shouldShowThinkingIndicator(turnPhase) && !showPlanPreview
	const showRegularStream = Boolean(streamingText) && !showPlanPreview
	const liveAssistantIndex =
		promptInFlight && messages[messages.length - 1]?.role === "assistant"
			? messages.length - 1
			: -1

	const isStaleReplay =
		replayingHistory && messages.length === 0 && lastRenderedMessagesRef.current.length > 0
	const sourceMessages = isStaleReplay ? lastRenderedMessagesRef.current : messages

	const visibleMessages =
		showPlanPreview && pendingPlan?.content.trim()
			? sourceMessages.filter((message) => {
					if (message.role !== "assistant") return true
					const content = message.content.trim()
					if (!content) return false
					const plan = pendingPlan.content.trim()
					return !(
						plan.startsWith(content) ||
						content.startsWith(plan) ||
						content === plan
					)
				})
			: sourceMessages

	return (
		<div
			ref={scrollRef}
			className="scrollbar-thin relative flex-1 overflow-y-auto px-4 py-4"
		>
			<MessageTrail
				items={trailItems}
				activeStore={activeStore}
				onSelect={scrollToMessage}
			/>
			{sourceMessages.length === 0 && !promptInFlight && !pendingPlan && !isStaleReplay ? (
				<div className="flex h-full items-center justify-center text-sm text-muted-foreground">
					{connected
						? "Envía tu primer mensaje al agente."
						: "Abre un proyecto desde el sidebar para empezar."}
				</div>
			) : null}

			<div
				className={cn(
					"mx-auto flex max-w-3xl flex-col gap-4 transition-opacity duration-150",
					isStaleReplay && "pointer-events-none opacity-40",
				)}
			>
				{visibleMessages.map((message, index) => {
					const isLiveAssistant =
						message.role === "assistant" &&
						index === liveAssistantIndex &&
						promptInFlight
					const toolCallsToShow = isLiveAssistant ? [] : message.toolCalls

					return (
						<div
							key={message.id}
							data-message-id={message.id}
							className={
								message.role === "user"
									? "message-trail-target ml-auto max-w-[85%] rounded-xl bg-secondary px-4 py-3 text-sm"
									: "message-trail-target max-w-full"
							}
						>
							{message.role === "assistant" ? (
								message.kind === "auth-request" ? (
									<AuthRequestCard message={message} />
								) : (
									<>
										<ToolCallList toolCalls={toolCallsToShow} />
										{message.content ? (
											<MarkdownContent content={message.content} />
										) : null}
									</>
								)
							) : (
								<p className="whitespace-pre-wrap">{message.content}</p>
							)}
						</div>
					)
				})}

				{showThinking ? <ThinkingIndicator active phase={turnPhase} /> : null}

				{promptInFlight && liveTurnTools.length > 0 ? (
					<ActivityTrace toolCalls={liveTurnTools} />
				) : null}

				{showPlanPreview ? (
					<div className="max-w-full">
						<PlanPreviewCard
							variant="embedded"
							content={planContent}
							isStreaming={promptInFlight && !pendingPlan}
							actionsEnabled={Boolean(pendingPlan) && !promptInFlight}
							onDownload={() => downloadContent(planContent)}
							onAccept={() => void acceptPlan()}
							onAcceptAndCompact={() => void acceptAndCompactPlan()}
							onComment={startPlanComment}
							onReject={() => void rejectPlan()}
						/>
					</div>
				) : null}

				{showRegularStream ? (
					<div className="max-w-full">
						<MarkdownContent content={streamingText} />
					</div>
				) : null}
			</div>
		</div>
	)
}