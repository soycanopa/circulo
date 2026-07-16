import { ChevronDown, ChevronRight, Loader2 } from "lucide-react"
import { useState } from "react"
import { ToolCallCard } from "@/components/tools/tool-call-card"
import { Badge } from "@/components/ui/badge"
import { useToolOverlay } from "@/hooks/use-tool-overlay"
import { groupStatus, type ToolCallGroup } from "@/lib/tool-call-groups"
import { hasMultiDiffTools } from "@/lib/tool-preview"
import { cn } from "@/lib/utils"

interface ToolCallGroupCardProps {
	group: ToolCallGroup
}

const statusLabel = {
	pending: "Pendiente",
	in_progress: "En progreso",
	completed: "Completado",
	failed: "Falló",
} as const

export function ToolCallGroupCard({ group }: ToolCallGroupCardProps) {
	const status = groupStatus(group.tools)
	const [open, setOpen] = useState(false)
	const { openMultiDiff } = useToolOverlay()
	const count = group.tools.length
	const showMultiDiff =
		(group.key === "edit" || group.key === "write") && hasMultiDiffTools(group.tools)

	if (count === 1) {
		return <ToolCallCard toolCall={group.tools[0]} />
	}

	return (
		<div className="my-2 rounded-md border border-border bg-card">
			<button
				type="button"
				onClick={() => setOpen((value) => !value)}
				className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm"
			>
				{open ? (
					<ChevronDown className="size-4 text-muted-foreground" />
				) : (
					<ChevronRight className="size-4 text-muted-foreground" />
				)}
				<span className="flex-1 font-medium">
					{group.label}
					<span className="ml-1.5 text-muted-foreground">({count})</span>
				</span>
				<Badge
					className={cn(
						status === "failed" && "border-destructive text-destructive",
						status === "completed" && "border-diff-addition text-diff-addition",
					)}
				>
					{status === "in_progress" || status === "pending" ? (
						<Loader2 className="mr-1 inline size-3 animate-spin" />
					) : null}
					{statusLabel[status]}
				</Badge>
			</button>

			{open ? (
				<div className="space-y-1 border-t border-border px-2 py-2">
					{showMultiDiff ? (
						<button
							type="button"
							onClick={() => openMultiDiff(group.tools)}
							className="mb-1 w-full rounded-md border border-border/60 px-2 py-1.5 text-left text-[11px] text-muted-foreground transition-colors hover:bg-accent/50 hover:text-foreground"
						>
							Ver todos los diffs ({group.tools.filter((tool) => tool.diff).length})
						</button>
					) : null}
					{group.tools.map((tool) => (
						<ToolCallCard key={tool.id} toolCall={tool} nested />
					))}
				</div>
			) : null}
		</div>
	)
}