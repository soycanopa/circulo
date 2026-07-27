import type { ToolCall } from "@/types/acp"
import { cn } from "@/lib/utils"

export function ToolCallCard({ tool }: { tool: ToolCall }) {
	return (
		<div className="overflow-hidden rounded-md border border-border bg-surface/80">
			<div className="flex items-center justify-between gap-2 px-2.5 py-1.5 text-xs">
				<span className="truncate font-medium text-fg/90">{tool.title}</span>
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
			</div>
			{tool.content ? (
				<pre className="max-h-40 overflow-auto border-t border-border px-2.5 py-2 font-mono text-[11px] text-muted">
					{tool.content.slice(0, 4000)}
				</pre>
			) : null}
		</div>
	)
}
