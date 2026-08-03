import type { ToolCall } from "@/types/acp"
import { cn } from "@/lib/utils"
import { isDiffTool } from "@/lib/diff-tools"
import { toolContentToText } from "@/lib/acp-parser"

interface ToolCallCardProps {
	tool: ToolCall
	onOpenDiff?: (tool: ToolCall) => void
}

export function ToolCallCard({ tool, onOpenDiff }: ToolCallCardProps) {
	const diff = isDiffTool(tool)

	return (
		<div
			className={cn(
				"overflow-hidden rounded-md border bg-surface/80",
				diff ? "border-sky-500/25" : "border-border",
			)}
		>
			<button
				type="button"
				disabled={!diff || !onOpenDiff}
				onClick={() => diff && onOpenDiff?.(tool)}
				className={cn(
					"flex w-full items-center justify-between gap-2 px-2.5 py-1.5 text-left text-xs",
					diff && onOpenDiff && "cursor-pointer hover:bg-white/5",
				)}
			>
				<span className="truncate font-medium text-fg/90">
					{diff ? `Diff · ${tool.title}` : tool.title}
				</span>
				<span
					className={cn(
						"shrink-0 rounded px-1.5 py-0.5 text-[10px] uppercase tracking-wide",
						tool.status === "completed" && "bg-emerald-500/15 text-emerald-300",
						tool.status === "failed" && "bg-red-500/15 text-red-300",
						tool.status !== "completed" &&
							tool.status !== "failed" &&
							"bg-white/5 text-muted",
					)}
				>
					{tool.status}
				</span>
			</button>
			{tool.content ? (
				<pre
					className={cn(
						"max-h-48 overflow-auto border-t px-2.5 py-2 font-mono text-[11px] leading-relaxed",
						diff
							? "border-sky-500/20 bg-sky-500/5 text-sky-100"
							: "border-border text-muted",
					)}
				>
					{toolContentToText(tool.content).slice(0, 4000)}
				</pre>
			) : null}
			{diff && onOpenDiff ? (
				<div className="border-t border-sky-500/20 px-2.5 py-1 text-[10px] text-muted">
					Click header to open diff panel
				</div>
			) : null}
		</div>
	)
}
