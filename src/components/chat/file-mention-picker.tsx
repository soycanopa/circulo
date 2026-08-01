import { File } from "lucide-react"
import { useEffect, useState } from "react"
import { searchFiles } from "@/lib/tauri"
import { cn } from "@/lib/utils"

interface FileMentionPickerProps {
	query: string
	selectedIndex: number
	onSelect: (path: string) => void
	onResultsChange?: (paths: string[]) => void
}

export function FileMentionPicker({
	query,
	selectedIndex,
	onSelect,
	onResultsChange,
}: FileMentionPickerProps) {
	const [results, setResults] = useState<string[]>([])
	const [loading, setLoading] = useState(false)
	const [error, setError] = useState<string | null>(null)

	useEffect(() => {
		let cancelled = false
		setLoading(true)
		setError(null)

		const timer = window.setTimeout(() => {
			void searchFiles(query)
				.then((paths) => {
					if (cancelled) return
					const next = paths.slice(0, 8)
					setResults(next)
					onResultsChange?.(next)
				})
				.catch((err) => {
					if (cancelled) return
					setResults([])
					onResultsChange?.([])
					setError(
						err instanceof Error ? err.message : "Could not search files",
					)
				})
				.finally(() => {
					if (!cancelled) setLoading(false)
				})
		}, 120)

		return () => {
			cancelled = true
			window.clearTimeout(timer)
		}
	}, [query, onResultsChange])

	return (
		<div className="absolute bottom-full left-0 right-0 z-20 mb-1 overflow-hidden rounded-md border border-border bg-sidebar shadow-lg">
			<div className="border-b border-border px-2.5 py-1.5 text-[10px] uppercase tracking-wider text-muted">
				Files in project
			</div>
			{loading ? (
				<p className="px-2.5 py-2 text-xs text-muted">Searching…</p>
			) : error ? (
				<p className="px-2.5 py-2 text-xs text-red-300">{error}</p>
			) : results.length === 0 ? (
				<p className="px-2.5 py-2 text-xs text-muted">No matching files</p>
			) : (
				<ul className="max-h-48 overflow-y-auto py-1">
					{results.map((path, index) => (
						<li key={path}>
							<button
								type="button"
								onMouseDown={(event) => {
									// Keep focus on the textarea.
									event.preventDefault()
									onSelect(path)
								}}
								className={cn(
									"flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs text-fg transition",
									index === selectedIndex
										? "bg-white/10"
										: "hover:bg-white/5",
								)}
							>
								<File className="size-3.5 shrink-0 text-muted" />
								<span className="truncate font-mono">{path}</span>
							</button>
						</li>
					))}
				</ul>
			)}
		</div>
	)
}
