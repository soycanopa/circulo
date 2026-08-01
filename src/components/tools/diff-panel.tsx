import { X } from "lucide-react"
import { useAtom } from "jotai"
import { selectedDiffToolAtom } from "@/stores/atoms"

export function DiffPanel() {
	const [tool, setTool] = useAtom(selectedDiffToolAtom)

	if (!tool) return null

	return (
		<aside className="flex w-96 shrink-0 flex-col border-l border-border bg-sidebar">
			<div className="flex h-12 items-center justify-between border-b border-border px-4">
				<div className="min-w-0">
					<p className="truncate text-sm font-medium text-fg">Diff review</p>
					<p className="truncate text-xs text-muted">{tool.title}</p>
				</div>
				<button
					type="button"
					onClick={() => setTool(null)}
					className="rounded p-1 text-muted transition hover:bg-white/5 hover:text-fg"
					title="Close panel"
				>
					<X className="size-4" />
				</button>
			</div>
			<pre className="min-h-0 flex-1 overflow-auto px-4 py-3 font-mono text-[11px] leading-relaxed text-sky-100">
				{tool.content ?? "No diff content available."}
			</pre>
		</aside>
	)
}
