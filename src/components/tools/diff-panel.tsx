import { useAtom, useAtomValue, useSetAtom } from "jotai"
import {
	FileDiff,
	FileMinus2,
	FilePlus2,
	MessageSquareText,
	Plus,
	Send,
	Trash2,
	X,
} from "lucide-react"
import { useState } from "react"
import { collectSessionDiffs, type SessionDiff } from "@/lib/diff-tools"
import { cn } from "@/lib/utils"
import {
	appendComposerTextAtom,
	pendingCommentsAtom,
	selectedDiffToolAtom,
	visibleMessagesAtom,
} from "@/stores/atoms"

interface DiffPanelProps {
	onClose: () => void
}

const STATUS_LABEL: Record<SessionDiff["status"], string> = {
	created: "created",
	modified: "modified",
	deleted: "deleted",
	unchanged: "unchanged",
}

export function DiffPanel({ onClose }: DiffPanelProps) {
	const messages = useAtomValue(visibleMessagesAtom)
	const [tool, setTool] = useAtom(selectedDiffToolAtom)
	const [collapsed, setCollapsed] = useState<Set<string>>(new Set())
	const pendingComments = useAtomValue(pendingCommentsAtom)
	const setPendingComments = useSetAtom(pendingCommentsAtom)
	const appendComposerText = useSetAtom(appendComposerTextAtom)
	const diffs = collectSessionDiffs(messages)

	const selectedPath =
		tool && typeof tool.content === "object"
			? (tool.content as { path?: string }).path
			: null

	// `selectedDiffToolAtom` may point to a raw tool call (opened from the chat);
	// resolve it to the matching aggregated file when it has a path.
	const selectedDiff =
		(typeof tool?.content === "object" &&
			tool.content?.type === "diff" &&
			diffs.find((d) => d.path === (tool.content as { path: string }).path)) ??
		(selectedPath ? diffs.find((d) => d.path === selectedPath) : null) ??
		null

	function toggleCollapse(path: string) {
		setCollapsed((prev) => {
			const next = new Set(prev)
			if (next.has(path)) next.delete(path)
			else next.add(path)
			return next
		})
	}

	function addComment(diff: SessionDiff, line: number, text: string) {
		const trimmed = text.trim()
		if (!trimmed) return
		setPendingComments((prev) => [
			...prev,
			{ path: diff.path, line, text: trimmed },
		])
	}

	function sendFeedback() {
		if (pendingComments.length === 0) return
		const body = pendingComments
			.map((comment) => `- ${comment.path}:${comment.line}: ${comment.text}`)
			.join("\n")
		appendComposerText(`Feedback on the current diff:\n${body}`)
		setPendingComments([])
	}

	const created = diffs.filter((d) => d.status === "created").length
	const modified = diffs.filter((d) => d.status === "modified").length

	return (
		<aside className="flex h-full w-full flex-col overflow-hidden rounded-tr-[8px] rounded-br-[8px]">
			<div
				className="flex h-12 shrink-0 items-center justify-between border-b border-border px-4 pb-0.5"
				data-tauri-drag-region="deep"
			>
				<div className="flex min-w-0 items-center gap-2">
					<FileDiff className="size-4 shrink-0 text-sky-300" />
					<div className="min-w-0">
						<p className="truncate text-sm font-medium text-fg">Diff review</p>
						{selectedDiff ? (
							<p className="truncate text-xs text-muted">{selectedDiff.path}</p>
						) : diffs.length > 0 ? (
							<p className="truncate text-xs text-muted">
								{created} added · {modified} modified
							</p>
						) : null}
					</div>
				</div>
				<button
					type="button"
					onClick={onClose}
					className="rounded p-1 text-muted transition hover:bg-white/5 hover:text-fg"
					title="Close diff panel"
					data-tauri-drag-region="false"
				>
					<X className="size-4" />
				</button>
			</div>

			{selectedDiff ? (
				<DiffView
					diff={selectedDiff}
					onBack={() => setTool(null)}
					onAddComment={(line, text) => addComment(selectedDiff, line, text)}
				/>
			) : diffs.length > 0 ? (
				<ul className="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto p-3">
					{diffs.map((diff) => {
						const isCollapsed = collapsed.has(diff.path)
						return (
							<li key={diff.path}>
								<button
									type="button"
									onClick={() => {
										if (isCollapsed) {
											toggleCollapse(diff.path)
											return
										}
										setTool({
											id: `session-${diff.path}`,
											title: diff.path,
											status: "completed",
											kind: "diff",
											content: {
												type: "diff",
												path: diff.path,
												oldText: diff.oldText,
												newText: diff.newText,
											},
										})
									}}
									className={cn(
										"w-full rounded-md border px-3 py-2 text-left text-xs transition",
										"border-sky-500/20 bg-sky-500/5 text-fg hover:bg-sky-500/10",
									)}
								>
									<span className="flex items-center gap-1.5">
										{diff.status === "created" ? (
											<FilePlus2 className="size-3.5 shrink-0 text-emerald-300" />
										) : diff.status === "deleted" ? (
											<FileMinus2 className="size-3.5 shrink-0 text-red-300" />
										) : (
											<FileDiff className="size-3.5 shrink-0 text-sky-300" />
										)}
										<span className="min-w-0 flex-1 truncate font-medium">
											{diff.path}
										</span>
									</span>
									<span className="mt-1 flex items-center justify-between gap-2">
										<span className="text-[10px] uppercase tracking-wide text-muted">
											{STATUS_LABEL[diff.status]}
											{diff.generated ? " · generated" : ""}
										</span>
										{diff.generated && !isCollapsed ? (
											<span className="text-[10px] text-muted">
												click to collapse
											</span>
										) : null}
									</span>
								</button>
							</li>
						)
					})}
				</ul>
			) : (
				<div className="flex min-h-0 flex-1 items-center justify-center px-6 text-center text-xs text-muted">
					No file changes in this session yet.
				</div>
			)}

			{pendingComments.length > 0 ? (
				<div className="shrink-0 border-t border-border bg-amber-500/5 px-3 py-2">
					<p className="flex items-center gap-1.5 text-[11px] text-amber-200">
						<MessageSquareText className="size-3.5" />
						{pendingComments.length} comment
						{pendingComments.length > 1 ? "s" : ""} ready to send
					</p>
					<div className="mt-1.5 flex items-center gap-1.5">
						<button
							type="button"
							onClick={sendFeedback}
							className="inline-flex items-center gap-1.5 rounded-md border border-amber-500/40 bg-amber-500/10 px-2 py-1 text-[11px] text-amber-100 transition hover:bg-amber-500/20"
						>
							<Send className="size-3" />
							Send feedback
						</button>
						<button
							type="button"
							onClick={() => setPendingComments([])}
							className="inline-flex items-center gap-1.5 rounded-md border border-border px-2 py-1 text-[11px] text-muted transition hover:bg-white/5 hover:text-fg"
						>
							<Trash2 className="size-3" />
							Clear
						</button>
					</div>
				</div>
			) : null}
		</aside>
	)
}

function DiffView({
	diff,
	onBack,
	onAddComment,
}: {
	diff: SessionDiff
	onBack: () => void
	onAddComment: (line: number, text: string) => void
}) {
	return (
		<div className="flex min-h-0 flex-1 flex-col">
			<div className="flex shrink-0 items-center justify-between border-b border-border px-4 py-2">
				<p className="min-w-0 truncate font-mono text-[11px] text-sky-200">
					{diff.path}
				</p>
				<button
					type="button"
					onClick={onBack}
					className="shrink-0 rounded px-2 py-0.5 text-[11px] text-muted transition hover:bg-white/5 hover:text-fg"
				>
					Back to files
				</button>
			</div>
			<div className="min-h-0 flex-1 overflow-auto px-4 py-3 font-mono text-[11px] leading-relaxed text-sky-100">
				<div className="flex flex-col gap-3">
					<div>
						<p className="font-sans text-[10px] uppercase tracking-wide text-muted">
							Old
						</p>
						<pre className="mt-1 whitespace-pre-wrap break-words rounded bg-black/30 px-2 py-1 text-red-200">
							{diff.oldText || "(empty)"}
						</pre>
					</div>
					<div>
						<p className="font-sans text-[10px] uppercase tracking-wide text-muted">
							New
						</p>
						<div className="mt-1 rounded bg-black/30 py-1 text-emerald-200">
							{diff.newText
								? diff.newText.split("\n").map((line, index) => (
										<div
											key={index}
											className="group flex items-start gap-1.5 px-2 hover:bg-emerald-500/5"
										>
											<span className="w-6 shrink-0 select-none text-right text-[10px] text-emerald-200/40">
												{index + 1}
											</span>
											<button
												type="button"
												title={`Comment on line ${index + 1}`}
												onClick={() => onAddComment(index + 1, line)}
												className="mt-0.5 shrink-0 rounded p-0.5 text-emerald-200/0 transition group-hover:text-emerald-200/70 hover:!text-emerald-100"
											>
												<Plus className="size-3" />
											</button>
											<span className="min-w-0 flex-1 whitespace-pre-wrap break-words">
												{line || " "}
											</span>
										</div>
									))
								: "(empty)"}
						</div>
					</div>
				</div>
			</div>
		</div>
	)
}
