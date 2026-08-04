import { ChevronRight } from "lucide-react"
import { useState } from "react"
import type { ToolCall } from "@/types/acp"
import { cn } from "@/lib/utils"
import { isDiffTool } from "@/lib/diff-tools"
import { isTerminalTool, terminalIdFromTool } from "@/lib/terminal-tools"
import { toolContentToText } from "@/lib/acp-parser"

interface ToolCallCardProps {
	tool: ToolCall
	onOpenDiff?: (tool: ToolCall) => void
	onOpenTerminal?: (terminalId: string) => void
}

export function ToolCallCard({ tool, onOpenDiff, onOpenTerminal }: ToolCallCardProps) {
	const [open, setOpen] = useState(false)
	const diff = isDiffTool(tool)
	const terminal = isTerminalTool(tool)
	const terminalId = terminalIdFromTool(tool)
	const detailText = tool.content ? toolContentToText(tool.content) : ""

	return (
		<div
			className={cn(
				"overflow-hidden rounded-md border bg-surface/80",
				diff
					? "border-sky-500/25"
					: terminal
						? "border-emerald-500/25"
						: "border-border",
			)}
		>
			<button
				type="button"
				onClick={() => setOpen((value) => !value)}
				aria-expanded={open}
				className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs hover:bg-white/5"
			>
				<ChevronRight
					className={cn(
						"size-3.5 shrink-0 text-muted transition-transform",
						open && "rotate-90",
					)}
				/>
				<span className="min-w-0 flex-1 truncate font-medium text-fg/90">
					{diff ? `Diff · ${tool.title}` : terminal ? `Terminal · ${tool.title}` : tool.title}
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
			{open ? (
				<div
					className={cn(
						"border-t",
						diff
							? "border-sky-500/20"
							: terminal
								? "border-emerald-500/20"
								: "border-border",
					)}
				>
					{detailText ? (
						<pre
							className={cn(
								"max-h-48 overflow-auto px-2.5 py-2 font-mono text-[11px] leading-relaxed",
								diff
									? "bg-sky-500/5 text-sky-100"
									: terminal
										? "bg-emerald-500/5 text-emerald-100"
										: "text-muted",
							)}
						>
							{detailText.slice(0, 4000)}
						</pre>
					) : (
						<p className="px-2.5 py-2 text-[11px] text-muted">No output yet.</p>
					)}
					{terminal && terminalId && onOpenTerminal ? (
						<div className="border-t border-emerald-500/20 px-2.5 py-1.5">
							<button
								type="button"
								onClick={() => onOpenTerminal(terminalId)}
								className="text-[10px] text-emerald-300/90 underline-offset-2 hover:text-emerald-200 hover:underline"
							>
								Open terminal drawer
							</button>
						</div>
					) : null}
					{diff && onOpenDiff ? (
						<div className="border-t border-sky-500/20 px-2.5 py-1.5">
							<button
								type="button"
								onClick={() => onOpenDiff(tool)}
								className="text-[10px] text-sky-300/90 underline-offset-2 hover:text-sky-200 hover:underline"
							>
								Open diff panel
							</button>
						</div>
					) : null}
				</div>
			) : null}
		</div>
	)
}
