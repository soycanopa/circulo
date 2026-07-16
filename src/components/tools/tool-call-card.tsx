import AnsiToHtml from "ansi-to-react"
import { ChevronDown, ChevronRight, Expand, Loader2 } from "lucide-react"
import { useState } from "react"
import { InlineDiffBlock } from "@/components/diff/inline-diff-block"
import { Badge } from "@/components/ui/badge"
import { useToolOverlay } from "@/hooks/use-tool-overlay"
import { canExpandTool } from "@/lib/tool-preview"
import { cn } from "@/lib/utils"
import type { ToolCallState } from "@/types/acp"

interface ToolCallCardProps {
	toolCall: ToolCallState
	nested?: boolean
}

const statusLabel: Record<ToolCallState["status"], string> = {
	pending: "Pendiente",
	in_progress: "En progreso",
	completed: "Completado",
	failed: "Falló",
}

export function ToolCallCard({ toolCall, nested = false }: ToolCallCardProps) {
	const [open, setOpen] = useState(false)
	const { openTool } = useToolOverlay()
	const isActive = toolCall.status === "in_progress" || toolCall.status === "pending"
	const expandable = canExpandTool(toolCall)

	return (
		<div
			className={cn(
				"rounded-md border border-border bg-card",
				nested ? "my-0" : "my-2",
			)}
		>
			<div className="flex items-center gap-1 pr-1">
				<button
					type="button"
					onClick={() => setOpen((value) => !value)}
					className="flex min-w-0 flex-1 items-center gap-2 px-3 py-2 text-left text-sm"
				>
					{open ? (
						<ChevronDown className="size-4 text-muted-foreground" />
					) : (
						<ChevronRight className="size-4 text-muted-foreground" />
					)}
					<span className="min-w-0 flex-1 truncate font-medium">{toolCall.title}</span>
					{toolCall.kind ? <Badge className="shrink-0">{toolCall.kind}</Badge> : null}
					<Badge
						className={cn(
							"shrink-0",
							toolCall.status === "failed" && "border-destructive text-destructive",
							toolCall.status === "completed" && "border-diff-addition text-diff-addition",
						)}
					>
						{isActive ? <Loader2 className="mr-1 inline size-3 animate-spin" /> : null}
						{statusLabel[toolCall.status]}
					</Badge>
				</button>
				{expandable ? (
					<button
						type="button"
						onClick={() => openTool(toolCall)}
						className="rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
						aria-label="Expandir resultado"
					>
						<Expand className="size-3.5" />
					</button>
				) : null}
			</div>

			{open ? (
				<div className="border-t border-border px-3 py-2 text-xs text-muted-foreground">
					{toolCall.diff ? (
						<InlineDiffBlock
							path={toolCall.diff.path}
							oldText={toolCall.diff.oldText}
							newText={toolCall.diff.newText}
						/>
					) : null}

					{toolCall.content ? (
						<div className="overflow-x-auto rounded-md bg-muted/40 p-2 font-mono whitespace-pre-wrap">
							{toolCall.kind === "execute" ? (
								<AnsiToHtml>{toolCall.content}</AnsiToHtml>
							) : (
								toolCall.content
							)}
						</div>
					) : null}
				</div>
			) : null}
		</div>
	)
}