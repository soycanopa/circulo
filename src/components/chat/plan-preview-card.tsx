import {
	ChevronDown,
	Download,
	Expand,
	Loader2,
	MessageSquare,
	ThumbsDown,
	ThumbsUp,
} from "lucide-react"
import { useSetAtom } from "jotai"
import { useRef, useState } from "react"
import { MarkdownContent } from "@/components/chat/markdown-content"
import { SelectorMenuItem } from "@/components/chat/selector-menu-item"
import { SelectorPortalMenu } from "@/components/chat/selector-portal-menu"
import { normalizePlanMarkdown } from "@/lib/plan-markdown"
import { planOverlayAtom } from "@/stores/atoms"
import { cn } from "@/lib/utils"

interface PlanPreviewCardProps {
	content: string
	isStreaming?: boolean
	actionsEnabled?: boolean
	variant?: "standalone" | "embedded"
	showExpand?: boolean
	onDownload: () => void
	onAccept: () => void
	onAcceptAndCompact: () => void
	onComment: () => void
	onReject: () => void
}

export function PlanPreviewCard({
	content,
	isStreaming = false,
	actionsEnabled = false,
	variant = "standalone",
	showExpand = true,
	onDownload,
	onAccept,
	onAcceptAndCompact,
	onComment,
	onReject,
}: PlanPreviewCardProps) {
	const normalizedContent = normalizePlanMarkdown(content)
	const canAct = actionsEnabled && !isStreaming && Boolean(normalizedContent.trim())
	const [acceptMenuOpen, setAcceptMenuOpen] = useState(false)
	const acceptMenuRef = useRef<HTMLDivElement>(null)
	const setPlanOverlay = useSetAtom(planOverlayAtom)
	const isEmbedded = variant === "embedded"

	function openFullscreen() {
		if (!normalizedContent.trim()) return
		setPlanOverlay({
			content: normalizedContent,
			isStreaming,
			actionsEnabled: canAct,
			onDownload,
			onAccept,
			onAcceptAndCompact,
			onComment,
			onReject,
		})
	}

	return (
		<div
			className={cn(
				"overflow-hidden rounded-xl border bg-card shadow-sm",
				isEmbedded
					? "border-diff-addition/35"
					: "border-[#3B5EF9]/30",
			)}
		>
			<div
				className={cn(
					"flex items-center justify-between gap-2 border-b px-4 py-2.5",
					isEmbedded
						? "border-diff-addition/20 bg-diff-addition/10"
						: "border-border/50",
				)}
			>
				<div className="flex items-center gap-2 text-sm font-medium text-foreground">
					{isStreaming ? (
						<Loader2
							className={cn(
								"size-3.5 animate-spin",
								isEmbedded ? "text-diff-addition" : "text-[#3B5EF9]",
							)}
						/>
					) : (
						<span
							className={cn(
								"size-2 rounded-full",
								isEmbedded ? "bg-diff-addition" : "bg-[#3B5EF9]",
							)}
						/>
					)}
					<span>Plan propuesto</span>
				</div>
				<div className="flex items-center gap-1">
					{showExpand ? (
						<button
							type="button"
							disabled={!normalizedContent.trim()}
							onClick={openFullscreen}
							className="inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground disabled:opacity-40"
						>
							<Expand className="size-3.5" />
							Expandir
						</button>
					) : null}
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
			</div>

			<div className="plan-preview-scroll relative max-h-[min(60vh,28rem)] overflow-y-auto px-4 py-3">
				{normalizedContent.trim() ? (
					<MarkdownContent
						content={normalizedContent}
						className="prose-headings:text-foreground prose-p:text-foreground/90 prose-li:text-foreground/90"
					/>
				) : (
					<p className="text-sm text-muted-foreground">Escribiendo el plan…</p>
				)}
			</div>

			{canAct ? (
				<div className="flex flex-wrap items-center gap-2 border-t border-border/50 px-4 py-3">
					<div ref={acceptMenuRef} className="relative inline-flex">
						<button
							type="button"
							onClick={onAccept}
							className={cn(
								"inline-flex items-center gap-1.5 rounded-l-md px-3 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90",
								isEmbedded ? "bg-diff-addition" : "bg-[#3B5EF9]",
							)}
						>
							<ThumbsUp className="size-3.5" />
							Aceptar
						</button>
						<button
							type="button"
							onClick={() => setAcceptMenuOpen((open) => !open)}
							className={cn(
								"inline-flex items-center rounded-r-md border-l border-white/20 px-1.5 py-1.5 text-white transition-opacity hover:opacity-90",
								isEmbedded ? "bg-diff-addition" : "bg-[#3B5EF9]",
							)}
							aria-label="Más opciones de aceptación"
						>
							<ChevronDown className="size-3.5" />
						</button>

						<SelectorPortalMenu
							open={acceptMenuOpen}
							anchorRef={acceptMenuRef}
							onClose={() => setAcceptMenuOpen(false)}
							minWidth={220}
							preferPlacement="above"
							className="p-1"
						>
							<ul>
								<li>
									<SelectorMenuItem
										onClick={() => {
											setAcceptMenuOpen(false)
											onAcceptAndCompact()
										}}
										className="flex-col items-start gap-0.5 py-2"
									>
										<span className="text-xs font-medium text-foreground">
											Aceptar y compactar
										</span>
										<span className="text-[10px] leading-snug text-muted-foreground">
											Resume la conversación antes de ejecutar el plan
										</span>
									</SelectorMenuItem>
								</li>
							</ul>
						</SelectorPortalMenu>
					</div>

					<button
						type="button"
						onClick={onComment}
						className="inline-flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-xs transition-colors hover:bg-accent"
					>
						<MessageSquare className="size-3.5" />
						Comentar
					</button>
					<button
						type="button"
						onClick={onReject}
						className="inline-flex items-center gap-1.5 rounded-md border border-destructive/40 px-3 py-1.5 text-xs text-destructive transition-colors hover:bg-destructive/10"
					>
						<ThumbsDown className="size-3.5" />
						Rechazar
					</button>
				</div>
			) : null}
		</div>
	)
}