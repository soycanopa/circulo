import {
	ChevronDown,
	ChevronRight,
	FilePenLine,
	Loader2,
	Terminal,
	Wrench,
	type LucideIcon,
} from "lucide-react"
import { useMemo, useState } from "react"
import { CollapsedDiffRow } from "@/components/tools/collapsed-diff-row"
import { CollapsedToolRow } from "@/components/tools/collapsed-tool-row"
import { DiffPanelOpenButton } from "@/components/tools/diff-panel-open-button"
import { ExpandPreviewFooter } from "@/components/tools/expand-preview-footer"
import { useDiffPanel } from "@/hooks/use-diff-panel"
import { countLineDiffStats } from "@/lib/session-diff-stats"
import {
	partitionToolsBySection,
	sectionStatus,
	TOOL_SECTION_LABELS,
	TOOL_SECTION_ORDER,
	type ToolSectionKey,
} from "@/lib/tool-sections"
import { SECTION_PREVIEW_MAX_ITEMS } from "@/lib/tool-activity-limits"
import { cn } from "@/lib/utils"
import { diffPanelOpenAtom } from "@/stores/atoms"
import { useAtomValue } from "jotai"
import type { ToolCallState } from "@/types/acp"

interface StructuredToolActivityProps {
	toolCalls: ToolCallState[]
	compact?: boolean
}

const SECTION_ICONS: Record<ToolSectionKey, LucideIcon> = {
	files: FilePenLine,
	commands: Terminal,
	other: Wrench,
}

const statusLabel = {
	pending: "Pendiente",
	in_progress: "En progreso",
	completed: "Listo",
	failed: "Error",
} as const

function sectionDiffStats(tools: ToolCallState[]) {
	return tools
		.filter((tool) => tool.diff)
		.reduce(
			(acc, tool) => {
				const stats = countLineDiffStats(tool.diff!.oldText, tool.diff!.newText)
				return {
					additions: acc.additions + stats.additions,
					deletions: acc.deletions + stats.deletions,
				}
			},
			{ additions: 0, deletions: 0 },
		)
}

function ToolSectionBlock({
	section,
	tools,
	compact,
}: {
	section: ToolSectionKey
	tools: ToolCallState[]
	compact?: boolean
}) {
	const status = sectionStatus(tools)
	const hasActive = status === "in_progress" || status === "pending"
	const [open, setOpen] = useState(hasActive)
	const [showAll, setShowAll] = useState(false)
	const { openDiffs } = useDiffPanel()
	const diffPanelOpen = useAtomValue(diffPanelOpenAtom)
	const SectionIcon = SECTION_ICONS[section]
	const fileDiffCount = tools.filter((tool) => tool.diff).length
	const showPanelAction = section === "files" && fileDiffCount > 0
	const sectionStats = useMemo(
		() => (showPanelAction ? sectionDiffStats(tools) : null),
		[showPanelAction, tools],
	)
	const hasOverflow = tools.length > SECTION_PREVIEW_MAX_ITEMS
	const visibleTools = showAll ? tools : tools.slice(0, SECTION_PREVIEW_MAX_ITEMS)

	const toggleOpen = () => {
		setOpen((value) => {
			if (value) setShowAll(false)
			return !value
		})
	}

	return (
		<div
			className={cn(
				"overflow-hidden rounded-lg border border-border/50 bg-muted/10 shadow-sm",
				compact && "shadow-none",
			)}
		>
			<div className="flex items-center gap-1 px-1 py-1">
				<button
					type="button"
					onClick={toggleOpen}
					className="flex min-w-0 flex-1 items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition-colors hover:bg-accent/30"
				>
					<span className="flex size-6 shrink-0 items-center justify-center rounded-md bg-background/60 text-muted-foreground">
						{open ? (
							<ChevronDown className="size-3.5" />
						) : (
							<ChevronRight className="size-3.5" />
						)}
					</span>
					<SectionIcon className="size-3.5 shrink-0 text-muted-foreground" />
					<span className="min-w-0 flex-1">
						<span className="block truncate font-medium text-foreground">
							{TOOL_SECTION_LABELS[section]}
						</span>
						<span className="block text-[10px] text-muted-foreground">
							{tools.length} {tools.length === 1 ? "acción" : "acciones"}
							{hasActive ? " · en curso" : status === "completed" ? " · listo" : ""}
						</span>
					</span>
					{hasActive ? (
						<Loader2 className="size-3.5 shrink-0 animate-spin text-muted-foreground" />
					) : (
						<span
							className={cn(
								"size-1.5 shrink-0 rounded-full bg-muted-foreground/40",
								status === "failed" && "bg-destructive",
								status === "completed" && "bg-diff-addition/80",
							)}
							title={statusLabel[status]}
						/>
					)}
				</button>
				{showPanelAction ? (
					<DiffPanelOpenButton
						onClick={() => openDiffs(tools)}
						title="Ver cambios en el panel (⌘⇧D)"
						ariaLabel="Abrir todos los cambios en el panel"
						active={diffPanelOpen}
						stats={sectionStats ?? undefined}
					/>
				) : null}
			</div>

			{open ? (
				<div className="space-y-1 border-t border-border/40 bg-background/20 px-1.5 py-1.5">
					{section === "files"
						? visibleTools.map((tool) => (
								<CollapsedDiffRow key={tool.id} toolCall={tool} />
							))
						: visibleTools.map((tool) => (
								<CollapsedToolRow key={tool.id} toolCall={tool} />
							))}
					{hasOverflow && !showAll ? (
						<ExpandPreviewFooter
							label={`Ver todos (${tools.length})`}
							onClick={() => setShowAll(true)}
						/>
					) : null}
					{hasOverflow && showAll ? (
						<ExpandPreviewFooter
							label="Mostrar menos"
							variant="collapse"
							onClick={() => setShowAll(false)}
						/>
					) : null}
				</div>
			) : null}
		</div>
	)
}

export function StructuredToolActivity({
	toolCalls,
	compact = false,
}: StructuredToolActivityProps) {
	if (toolCalls.length === 0) return null

	const sections = partitionToolsBySection(toolCalls)

	return (
		<div className={cn("space-y-2", compact && "space-y-1.5")}>
			{TOOL_SECTION_ORDER.map((section) => {
				const tools = sections[section]
				if (tools.length === 0) return null
				return (
					<ToolSectionBlock
						key={section}
						section={section}
						tools={tools}
						compact={compact}
					/>
				)
			})}
		</div>
	)
}