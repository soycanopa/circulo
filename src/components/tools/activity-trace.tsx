import {
	FilePenLine,
	FileText,
	Loader2,
	Search,
	Terminal,
	Wrench,
} from "lucide-react"
import { getToolGroupKey } from "@/lib/tool-call-groups"
import { canExpandTool, hasMultiDiffTools } from "@/lib/tool-preview"
import { useDiffPanel } from "@/hooks/use-diff-panel"
import { cn } from "@/lib/utils"
import type { ToolCallState } from "@/types/acp"

interface ActivityTraceProps {
	toolCalls: ToolCallState[]
}

function TraceIcon({ toolCall }: { toolCall: ToolCallState }) {
	const key = getToolGroupKey(toolCall)
	const className = "size-3.5 shrink-0 text-muted-foreground"

	switch (key) {
		case "read":
			return <FileText className={className} />
		case "write":
		case "edit":
			return <FilePenLine className={className} />
		case "execute":
			return <Terminal className={className} />
		case "search":
		case "websearch":
			return <Search className={className} />
		default:
			return <Wrench className={className} />
	}
}

function ActivityTraceRow({
	toolCall,
	onOpen,
}: {
	toolCall: ToolCallState
	onOpen: () => void
}) {
	const isActive = toolCall.status === "pending" || toolCall.status === "in_progress"
	const isFailed = toolCall.status === "failed"
	const expandable = canExpandTool(toolCall)

	return (
		<button
			type="button"
			disabled={!expandable}
			onClick={onOpen}
			className={cn(
				"flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-xs text-muted-foreground transition-colors",
				expandable && "hover:bg-accent/50 hover:text-foreground",
				isFailed && "text-destructive/90",
				!expandable && "cursor-default",
			)}
		>
			<TraceIcon toolCall={toolCall} />
			<span className="min-w-0 flex-1 truncate">{toolCall.title}</span>
			{isActive ? (
				<Loader2 className="size-3 shrink-0 animate-spin opacity-70" />
			) : isFailed ? (
				<span className="shrink-0 text-[10px] uppercase tracking-wide">Error</span>
			) : expandable ? (
				<span className="shrink-0 text-[10px] uppercase tracking-wide opacity-60">
					Ver
				</span>
			) : (
				<span className="size-1.5 shrink-0 rounded-full bg-diff-addition/80" />
			)}
		</button>
	)
}

/** Compact in-turn tool activity (Craft-style trace, not full cards). */
export function ActivityTrace({ toolCalls }: ActivityTraceProps) {
	const { openDiff, openDiffs } = useDiffPanel()
	if (toolCalls.length === 0) return null

	const showMultiDiff = hasMultiDiffTools(toolCalls)

	return (
		<div className="rounded-md border border-border/60 bg-card/40 px-1 py-1">
			{toolCalls.map((toolCall) => (
				<ActivityTraceRow
					key={toolCall.id}
					toolCall={toolCall}
					onOpen={() => openDiff(toolCall)}
				/>
			))}
			{showMultiDiff ? (
				<button
					type="button"
					onClick={() => openDiffs(toolCalls)}
					className="mt-1 w-full rounded-md border border-border/60 px-2 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-accent/50 hover:text-foreground"
				>
					Ver todos los diffs ({toolCalls.filter((tool) => tool.diff).length})
				</button>
			) : null}
		</div>
	)
}