import AnsiToHtml from "ansi-to-react"
import { ChevronRight, Loader2, Terminal, Wrench } from "lucide-react"
import { useState } from "react"
import { ExpandPreviewFooter } from "@/components/tools/expand-preview-footer"
import { useToolOverlay } from "@/hooks/use-tool-overlay"
import { getToolGroupKey } from "@/lib/tool-call-groups"
import { INLINE_PREVIEW_MAX_HEIGHT_PX } from "@/lib/tool-activity-limits"
import { cn } from "@/lib/utils"
import type { ToolCallState } from "@/types/acp"

interface CollapsedToolRowProps {
	toolCall: ToolCallState
	defaultOpen?: boolean
}

export function CollapsedToolRow({
	toolCall,
	defaultOpen = false,
}: CollapsedToolRowProps) {
	const [open, setOpen] = useState(defaultOpen)
	const { openTool } = useToolOverlay()
	const isActive = toolCall.status === "pending" || toolCall.status === "in_progress"
	const isFailed = toolCall.status === "failed"
	const group = getToolGroupKey(toolCall)
	const Icon = group === "execute" ? Terminal : Wrench
	const hasContent = Boolean(toolCall.content.trim())

	const toggleOpen = () => {
		if (!hasContent) return
		setOpen((value) => !value)
	}

	return (
		<div
			className={cn(
				"overflow-hidden rounded-md border border-border/40 bg-background/50",
				isFailed && "border-destructive/40",
			)}
		>
			<button
				type="button"
				onClick={toggleOpen}
				className={cn(
					"flex w-full items-center gap-2 px-2.5 py-2 text-left text-xs transition-colors",
					hasContent && "hover:bg-accent/25",
					isFailed && "text-destructive/90",
					!hasContent && "cursor-default",
				)}
			>
				<ChevronRight
					className={cn(
						"size-3.5 shrink-0 text-muted-foreground transition-transform duration-150",
						open && "rotate-90",
						!hasContent && "opacity-30",
					)}
				/>
				<span className="flex size-5 shrink-0 items-center justify-center rounded bg-muted/50">
					<Icon className="size-3 text-muted-foreground" />
				</span>
				<span className="min-w-0 flex-1 truncate text-foreground">{toolCall.title}</span>
				{isActive ? (
					<Loader2 className="size-3 shrink-0 animate-spin text-muted-foreground" />
				) : isFailed ? (
					<span className="shrink-0 text-[10px] uppercase tracking-wide text-destructive">
						Error
					</span>
				) : null}
			</button>

			{open && hasContent ? (
				<div className="space-y-0.5 border-t border-border/40 bg-muted/10 px-2.5 py-2">
					<div
						className="scrollbar-thin overflow-y-auto rounded-md border border-border/30 bg-muted/30 p-2 font-mono text-[11px] whitespace-pre-wrap text-muted-foreground"
						style={{ maxHeight: `${INLINE_PREVIEW_MAX_HEIGHT_PX}px` }}
					>
						{group === "execute" ? (
							<AnsiToHtml>{toolCall.content}</AnsiToHtml>
						) : (
							toolCall.content
						)}
					</div>
					<ExpandPreviewFooter
						label="Ver salida completa"
						onClick={() => openTool(toolCall)}
					/>
				</div>
			) : null}
		</div>
	)
}