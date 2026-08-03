import { useAtom, useAtomValue } from "jotai"
import { FileDiff, X } from "lucide-react"
import { collectDiffTools } from "@/lib/diff-tools"
import { cn } from "@/lib/utils"
import { selectedDiffToolAtom, visibleMessagesAtom } from "@/stores/atoms"

interface DiffPanelProps {
	onClose: () => void
}

export function DiffPanel({ onClose }: DiffPanelProps) {
	const messages = useAtomValue(visibleMessagesAtom)
	const [tool, setTool] = useAtom(selectedDiffToolAtom)
	const diffTools = collectDiffTools(messages)

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
						{tool ? (
							<p className="truncate text-xs text-muted">{tool.title}</p>
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

			{tool ? (
				<div className="min-h-0 flex-1 overflow-auto px-4 py-3 font-mono text-[11px] leading-relaxed text-sky-100">
					{tool.content &&
					typeof tool.content === "object" &&
					tool.content.type === "diff" ? (
						<div className="flex flex-col gap-3">
							<p className="font-sans text-xs font-medium text-sky-200">
								{tool.content.path}
							</p>
							<div>
								<p className="font-sans text-[10px] uppercase tracking-wide text-muted">
									Old
								</p>
								<pre className="mt-1 whitespace-pre-wrap break-words rounded bg-black/30 px-2 py-1 text-red-200">
									{tool.content.oldText || "(empty)"}
								</pre>
							</div>
							<div>
								<p className="font-sans text-[10px] uppercase tracking-wide text-muted">
									New
								</p>
								<pre className="mt-1 whitespace-pre-wrap break-words rounded bg-black/30 px-2 py-1 text-emerald-200">
									{tool.content.newText || "(empty)"}
								</pre>
							</div>
						</div>
					) : (
						<pre className="whitespace-pre-wrap break-words">
							{typeof tool.content === "string"
								? tool.content
								: "No diff content available."}
						</pre>
					)}
				</div>
			) : diffTools.length > 0 ? (
				<ul className="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto p-3">
					{diffTools.map((item) => (
						<li key={item.id}>
							<button
								type="button"
								onClick={() => setTool(item)}
								className={cn(
									"w-full rounded-md border px-3 py-2 text-left text-xs transition",
									"border-sky-500/20 bg-sky-500/5 text-fg hover:bg-sky-500/10",
								)}
							>
								<span className="line-clamp-2 font-medium">{item.title}</span>
								<span className="mt-0.5 block text-[10px] uppercase tracking-wide text-muted">
									{item.status}
								</span>
							</button>
						</li>
					))}
				</ul>
			) : (
				<div className="min-h-0 flex-1" />
			)}
		</aside>
	)
}
