import { useAtomValue, useSetAtom } from "jotai"
import { QuestionCard } from "@/components/tools/question-card"
import { SubAgentCard } from "@/components/tools/sub-agent-card"
import { ToolCallCard } from "@/components/tools/tool-call-card"
import {
	MessageScroller,
	MessageScrollerButton,
	MessageScrollerContent,
	MessageScrollerItem,
	MessageScrollerProvider,
	MessageScrollerViewport,
} from "@/components/ui/message-scroller"
import { agentTerminalTabId } from "@/components/terminal/terminal-drawer"
import { ThinkingShimmer } from "@/components/chat/thinking-shimmer"
import { Markdown } from "@/components/chat/markdown"
import { terminalIdFromTool } from "@/lib/terminal-tools"
import {
	diffPanelOpenAtom,
	selectedDiffToolAtom,
	visibleMessagesAtom,
	visiblePromptInFlightAtom,
	visibleStreamingAtom,
	activeTerminalIdAtom,
	terminalDrawerOpenAtom,
} from "@/stores/atoms"
import type { ChatMessage, ToolCall } from "@/types/acp"

function MessageRow({
	message,
	isLiveAssistant,
	showCaret,
	onOpenDiff,
	onOpenTerminal,
}: {
	message: ChatMessage
	isLiveAssistant: boolean
	showCaret: boolean
	onOpenDiff: (tool: ToolCall) => void
	onOpenTerminal: (tool: ToolCall) => void
}) {
	return (
		<div className="mx-auto max-w-3xl">
			<div className="mb-1 text-[11px] uppercase tracking-wide text-muted">
				{message.role === "user" ? "You" : "Agent"}
			</div>
			{message.content ? (
				<div className="text-sm leading-relaxed text-fg">
					{message.role === "assistant" ? (
						<Markdown text={message.content} streaming={isLiveAssistant} />
					) : (
						<p className="whitespace-pre-wrap">{message.content}</p>
					)}
					{isLiveAssistant ? (
						<span className="ml-0.5 inline-block h-3.5 w-1.5 translate-y-0.5 animate-pulse bg-fg/70" />
					) : null}
				</div>
			) : message.role === "assistant" && showCaret ? (
				<ThinkingShimmer />
			) : null}
			{message.toolCalls.length > 0 ? (
				<div className="mt-2 space-y-1.5">
				{message.toolCalls.map((tool) => {
					if (tool.kind === "task") {
						return <SubAgentCard key={tool.id} tool={tool} />
					}
					if (tool.kind === "question") {
						return <QuestionCard key={tool.id} tool={tool} />
					}
					return (
						<ToolCallCard
							key={tool.id}
							tool={tool}
							onOpenDiff={onOpenDiff}
							onOpenTerminal={() => onOpenTerminal(tool)}
						/>
					)
				})}
				</div>
			) : null}
		</div>
	)
}

export function MessageList() {
	const messages = useAtomValue(visibleMessagesAtom)
	const streaming = useAtomValue(visibleStreamingAtom)
	const inFlight = useAtomValue(visiblePromptInFlightAtom)
	const setSelectedDiff = useSetAtom(selectedDiffToolAtom)
	const setDiffPanelOpen = useSetAtom(diffPanelOpenAtom)
	const setActiveTerminalId = useSetAtom(activeTerminalIdAtom)
	const setTerminalDrawerOpen = useSetAtom(terminalDrawerOpenAtom)

	function openDiffPanel(tool: ToolCall) {
		setSelectedDiff(tool)
		setDiffPanelOpen(true)
	}

	function openTerminalDrawer(tool: ToolCall) {
		const terminalId = terminalIdFromTool(tool)
		if (!terminalId) return
		setActiveTerminalId(agentTerminalTabId(terminalId))
		setTerminalDrawerOpen(true)
	}

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
		<MessageScrollerProvider autoScroll scrollPreviousItemPeek={64}>
			<MessageScroller className="flex-1 min-h-0">
				<MessageScrollerViewport className="px-4 py-4">
					<MessageScrollerContent className="gap-4">
						{messages.map((message) => {
							const isLiveAssistant =
								inFlight &&
								message.role === "assistant" &&
								message.id === last?.id
							return (
								<MessageScrollerItem
									key={message.id}
									messageId={message.id}
									scrollAnchor={message.role === "user"}
								>
									<MessageRow
										message={message}
										isLiveAssistant={isLiveAssistant}
										showCaret={showCaret && message.id === last?.id}
										onOpenDiff={openDiffPanel}
										onOpenTerminal={openTerminalDrawer}
									/>
								</MessageScrollerItem>
							)
						})}
						{streaming ? (
							<MessageScrollerItem messageId="__streaming__">
								<div className="mx-auto max-w-3xl">
									<div className="mb-1 text-[11px] uppercase tracking-wide text-muted">
										Agent
									</div>
									<div className="text-sm leading-relaxed text-fg">
										<Markdown text={streaming} streaming />
										<span className="ml-0.5 inline-block h-3.5 w-1.5 translate-y-0.5 animate-pulse bg-fg/70" />
									</div>
								</div>
							</MessageScrollerItem>
						) : null}
					</MessageScrollerContent>
				</MessageScrollerViewport>
				<MessageScrollerButton direction="end" />
			</MessageScroller>
		</MessageScrollerProvider>
	)
}
