import { Download, Loader2, MessageSquare, ThumbsDown, ThumbsUp } from "lucide-react"
import { MarkdownContent } from "@/components/chat/markdown-content"
import { normalizePlanMarkdown } from "@/lib/plan-markdown"
import { cn } from "@/lib/utils"

interface PlanPreviewCardProps {
	content: string
	isStreaming?: boolean
	actionsEnabled?: boolean
	onDownload: () => void
	onAccept: () => void
	onComment: () => void
	onReject: () => void
}

export function PlanPreviewCard({
	content,
	isStreaming = false,
	actionsEnabled = false,
	onDownload,
	onAccept,
	onComment,
	onReject,
}: PlanPreviewCardProps) {
	const normalizedContent = normalizePlanMarkdown(content)
	const canAct = actionsEnabled && !isStreaming && Boolean(normalizedContent.trim())

	return (
		<div className="overflow-hidden rounded-xl border border-[#3B5EF9]/30 bg-card shadow-sm">
			<div className="flex items-center justify-between gap-2 border-b border-border/50 px-4 py-2.5">
				<div className="flex items-center gap-2 text-sm font-medium text-foreground">
					{isStreaming ? (
						<Loader2 className="size-3.5 animate-spin text-[#3B5EF9]" />
					) : (
						<span className="size-2 rounded-full bg-[#3B5EF9]" />
					)}
					<span>Plan propuesto</span>
				</div>
				<button
					type="button"
					disabled={!normalizedContent.trim()}
					onClick={onDownload}
					className="inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground disabled:opacity-40"
				>
					<Download className="size-3.5" />
					Descargar
				</button>
			</div>

			<div className="max-h-[min(60vh,28rem)] overflow-y-auto px-4 py-3">
				{normalizedContent.trim() ? (
					<MarkdownContent content={normalizedContent} className="prose-headings:text-foreground prose-p:text-foreground/90 prose-li:text-foreground/90" />
				) : (
					<p className="text-sm text-muted-foreground">Escribiendo el plan…</p>
				)}
			</div>

			<div className="flex flex-wrap items-center gap-2 border-t border-border/50 px-4 py-3">
				<button
					type="button"
					disabled={!canAct}
					onClick={onAccept}
					className={cn(
						"inline-flex items-center gap-1.5 rounded-md bg-[#3B5EF9] px-3 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-40",
					)}
				>
					<ThumbsUp className="size-3.5" />
					Aceptar
				</button>
				<button
					type="button"
					disabled={!canAct}
					onClick={onComment}
					className="inline-flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-xs transition-colors hover:bg-accent disabled:opacity-40"
				>
					<MessageSquare className="size-3.5" />
					Comentar
				</button>
				<button
					type="button"
					disabled={!canAct}
					onClick={onReject}
					className="inline-flex items-center gap-1.5 rounded-md border border-destructive/40 px-3 py-1.5 text-xs text-destructive transition-colors hover:bg-destructive/10 disabled:opacity-40"
				>
					<ThumbsDown className="size-3.5" />
					Rechazar
				</button>
			</div>
		</div>
	)
}