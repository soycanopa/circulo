import { useSetAtom } from "jotai"
import { CheckCircle2, ChevronRight, ExternalLink, Loader2, XCircle } from "lucide-react"
import { useEffect, useState } from "react"
import { toolContentToText } from "@/lib/acp-parser"
import { setVisibleSession } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import { errorMessageAtom } from "@/stores/atoms"
import type { ToolCall } from "@/types/acp"

function sessionIdFromRawOutput(rawOutput: unknown): string | null {
	if (!rawOutput) return null
	if (typeof rawOutput === "string") {
		const match = /session[\s_-]?id"?\s*[:=]\s*"([^"]+)"/i.exec(rawOutput)
		return match?.[1] ?? null
	}
	if (typeof rawOutput === "object") {
		const record = rawOutput as Record<string, unknown>
		const id =
			record.sessionId ??
			record.session_id ??
			record.sessionID ??
			record.session
		return typeof id === "string" ? id : null
	}
	return null
}

const TASK_STATE_LABEL: Record<string, string> = {
	pending: "queued",
	running: "running",
	completed: "completed",
	failed: "failed",
}

export function SubAgentCard({ tool }: { tool: ToolCall }) {
	const [open, setOpen] = useState(false)
	const setError = useSetAtom(errorMessageAtom)
	const running = tool.taskState === "pending" || tool.taskState === "running"
	const done = tool.taskState === "completed" || tool.taskState === "failed"
	const childSessionId = sessionIdFromRawOutput(tool.rawOutput)
	const detailText = tool.content ? toolContentToText(tool.content) : ""

	// Auto-collapse once the sub-agent finishes.
	useEffect(() => {
		if (done) setOpen(false)
	}, [done])

	async function openChildSession(sessionId: string) {
		try {
			await setVisibleSession(sessionId)
		} catch (error) {
			setError(
				error instanceof Error ? error.message : "Failed to open child session",
			)
		}
	}

	return (
		<div
			className={cn(
				"overflow-hidden rounded-md border",
				tool.taskState === "failed"
					? "border-red-500/25"
					: running
						? "border-indigo-500/25"
						: "border-border",
			)}
		>
			<button
				type="button"
				onClick={() => setOpen((value) => !value)}
				aria-expanded={open}
				className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs hover:bg-white/5"
			>
				{running ? (
					<Loader2 className="size-3.5 shrink-0 animate-spin text-indigo-300" />
				) : tool.taskState === "failed" ? (
					<XCircle className="size-3.5 shrink-0 text-red-300" />
				) : (
					<CheckCircle2 className="size-3.5 shrink-0 text-emerald-300" />
				)}
				<span className="min-w-0 flex-1 truncate font-medium text-fg/90">
					Sub-agent · {tool.title}
				</span>
				<span
					className={cn(
						"shrink-0 rounded px-1.5 py-0.5 text-[10px] uppercase tracking-wide",
						tool.taskState === "failed" && "bg-red-500/15 text-red-300",
						tool.taskState === "completed" && "bg-emerald-500/15 text-emerald-300",
						running && "bg-indigo-500/15 text-indigo-300",
						!tool.taskState && "bg-white/5 text-muted",
					)}
				>
					{TASK_STATE_LABEL[tool.taskState ?? ""] ?? tool.status}
				</span>
				<ChevronRight
					className={cn(
						"size-3.5 shrink-0 text-muted transition-transform",
						open && "rotate-90",
					)}
				/>
			</button>
			{open ? (
				<div className="border-t border-border">
					{detailText ? (
						<pre className="max-h-48 overflow-auto px-2.5 py-2 font-mono text-[11px] leading-relaxed text-muted">
							{detailText.slice(0, 4000)}
						</pre>
					) : (
						<p className="px-2.5 py-2 text-[11px] text-muted">No output yet.</p>
					)}
					{childSessionId ? (
						<div className="border-t border-border px-2.5 py-1.5">
							<button
								type="button"
								onClick={() => void openChildSession(childSessionId)}
								className="inline-flex items-center gap-1 text-[10px] text-indigo-300/90 underline-offset-2 hover:text-indigo-200 hover:underline"
							>
								<ExternalLink className="size-3" />
								Open child session
							</button>
						</div>
					) : null}
				</div>
			) : null}
		</div>
	)
}
