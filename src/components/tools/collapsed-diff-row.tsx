import { ChevronRight, FilePenLine, Loader2 } from "lucide-react"
import { useState } from "react"
import { useAtomValue } from "jotai"
import { DiffStatLabel } from "@/components/chat/diff-stat-label"
import { InlineDiffBlock } from "@/components/diff/inline-diff-block"
import { DiffPanelOpenButton } from "@/components/tools/diff-panel-open-button"
import { ExpandPreviewFooter } from "@/components/tools/expand-preview-footer"
import { useDiffPanel } from "@/hooks/use-diff-panel"
import { countLineDiffStats } from "@/lib/session-diff-stats"
import { INLINE_PREVIEW_MAX_HEIGHT_PX } from "@/lib/tool-activity-limits"
import { pickPath } from "@/lib/tool-path"
import { cn } from "@/lib/utils"
import { activeDiffToolIdAtom, diffPanelOpenAtom } from "@/stores/atoms"
import type { ToolCallState } from "@/types/acp"

interface CollapsedDiffRowProps {
	toolCall: ToolCallState
	defaultOpen?: boolean
	showPanelAction?: boolean
}

function fileName(path: string) {
	return path.split("/").pop() ?? path
}

function fileDirectory(path: string) {
	const parts = path.split("/")
	parts.pop()
	return parts.join("/")
}

export function CollapsedDiffRow({
	toolCall,
	defaultOpen = false,
	showPanelAction = true,
}: CollapsedDiffRowProps) {
	const [open, setOpen] = useState(defaultOpen)
	const [fullPreview, setFullPreview] = useState(false)
	const { openDiff, openDiffFullscreen } = useDiffPanel()
	const diffPanelOpen = useAtomValue(diffPanelOpenAtom)
	const activeDiffToolId = useAtomValue(activeDiffToolIdAtom)
	const path = toolCall.diff?.path ?? pickPath(toolCall) ?? toolCall.title
	const stats = countLineDiffStats(toolCall.diff?.oldText, toolCall.diff?.newText ?? "")
	const isActive = toolCall.status === "pending" || toolCall.status === "in_progress"
	const isFailed = toolCall.status === "failed"
	const directory = fileDirectory(path)
	const name = fileName(path)
	const panelTargetsThisFile =
		diffPanelOpen && activeDiffToolId === toolCall.id

	const toggleOpen = () => {
		setOpen((value) => {
			if (value) setFullPreview(false)
			return !value
		})
	}

	const openFullDiff = () => {
		if (showPanelAction) {
			openDiffFullscreen(toolCall)
			return
		}
		setFullPreview(true)
	}

	return (
		<div
			className={cn(
				"overflow-hidden rounded-md border border-border/40 bg-background/50",
				isFailed && "border-destructive/40",
				panelTargetsThisFile && "ring-1 ring-border/80",
			)}
		>
			<div className="flex items-center gap-0.5 pr-0.5">
				<button
					type="button"
					onClick={toggleOpen}
					className={cn(
						"flex min-w-0 flex-1 items-center gap-2 px-2.5 py-2 text-left text-xs transition-colors hover:bg-accent/25",
						isFailed && "text-destructive/90",
					)}
				>
					<ChevronRight
						className={cn(
							"size-3.5 shrink-0 text-muted-foreground transition-transform duration-150",
							open && "rotate-90",
						)}
					/>
					<span className="flex size-5 shrink-0 items-center justify-center rounded bg-muted/50">
						<FilePenLine className="size-3 text-muted-foreground" />
					</span>
					<span className="min-w-0 flex-1">
						<span className="block truncate font-mono text-[11px] text-foreground">
							{name}
						</span>
						{directory ? (
							<span className="mt-0.5 block truncate text-[10px] text-muted-foreground">
								{directory}
							</span>
						) : null}
					</span>
					<DiffStatLabel
						additions={stats.additions}
						deletions={stats.deletions}
						className="shrink-0 text-[11px]"
					/>
					{isActive ? (
						<Loader2 className="size-3 shrink-0 animate-spin text-muted-foreground" />
					) : isFailed ? (
						<span className="shrink-0 text-[10px] uppercase tracking-wide text-destructive">
							Error
						</span>
					) : null}
				</button>
				{showPanelAction && toolCall.diff ? (
					<DiffPanelOpenButton
						onClick={() => openDiff(toolCall)}
						title="Ver en panel de cambios (⌘⇧D)"
						ariaLabel={`Abrir ${name} en el panel de cambios`}
						active={panelTargetsThisFile}
					/>
				) : null}
			</div>

			{open && toolCall.diff ? (
				<div className="space-y-0.5 border-t border-border/40 bg-muted/10 px-2 pb-2 pt-1">
					<div
						className={cn(
							"rounded-md",
							!fullPreview && "scrollbar-thin overflow-y-auto",
						)}
						style={
							fullPreview
								? undefined
								: { maxHeight: `${INLINE_PREVIEW_MAX_HEIGHT_PX}px` }
						}
					>
						<InlineDiffBlock
							path={toolCall.diff.path}
							oldText={toolCall.diff.oldText}
							newText={toolCall.diff.newText}
						/>
					</div>
					{!fullPreview ? (
						<ExpandPreviewFooter
							label="Ver diff completo"
							onClick={openFullDiff}
						/>
					) : (
						<ExpandPreviewFooter
							label="Mostrar menos"
							variant="collapse"
							onClick={() => setFullPreview(false)}
						/>
					)}
				</div>
			) : null}
		</div>
	)
}