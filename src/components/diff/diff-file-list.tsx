import { FileCode2, FileText } from "lucide-react"
import type { SessionDiffEntry } from "@/lib/session-diffs"
import { cn } from "@/lib/utils"

interface DiffFileListProps {
	entries: SessionDiffEntry[]
	activeId: string
	onSelect: (id: string) => void
}

function fileName(path: string) {
	return path.split("/").pop() ?? path
}

function fileDirectory(path: string) {
	const parts = path.split("/")
	parts.pop()
	return parts.join("/")
}

export function DiffFileList({ entries, activeId, onSelect }: DiffFileListProps) {
	if (entries.length === 0) return null

	return (
		<ul className="scrollbar-thin flex min-h-0 flex-col gap-0.5 overflow-y-auto p-1.5">
			{entries.map((entry) => {
				const isActive = entry.id === activeId
				const directory = fileDirectory(entry.path)
				const name = fileName(entry.path)
				const Icon = entry.path.match(/\.(md|txt)$/i) ? FileText : FileCode2

				return (
					<li key={entry.id}>
						<button
							type="button"
							onClick={() => onSelect(entry.id)}
							className={cn(
								"flex w-full min-w-0 items-start gap-2 rounded-md px-2 py-1.5 text-left transition-colors",
								isActive
									? "bg-[rgba(255,255,255,0.1)] text-foreground"
									: "text-muted-foreground hover:bg-[rgba(255,255,255,0.05)] hover:text-foreground",
							)}
						>
							<Icon className="mt-0.5 size-3.5 shrink-0 opacity-70" />
							<span className="min-w-0 flex-1">
								<span className="block truncate font-mono text-[11px] leading-tight">
									{name}
								</span>
								{directory ? (
									<span className="mt-0.5 block truncate text-[10px] opacity-70">
										{directory}
									</span>
								) : null}
							</span>
						</button>
					</li>
				)
			})}
		</ul>
	)
}