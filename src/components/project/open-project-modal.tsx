import { Folder, FolderOpen, FolderSearch, HardDrive } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import {
	completeDirectoryPath,
	type DirectoryCompletion,
} from "@/lib/tauri"
import { cn } from "@/lib/utils"
import type { RecentProject } from "@/types/acp"

function projectLabel(path: string): string {
	if (path.includes("/.circulo/chats")) return "General Chat"
	const parts = path.split("/").filter(Boolean)
	return parts.slice(-2).join("/") || path
}

/** True when the query looks like a filesystem path we can complete. */
function isPathQuery(value: string): boolean {
	const t = value.trim()
	return t.startsWith("/") || t === "~" || t.startsWith("~/")
}

type PaletteRow =
	| {
			id: string
			kind: "recent"
			path: string
			label: string
			subtitle: string
			isCurrent: boolean
	  }
	| {
			id: string
			kind: "fs"
			path: string
			label: string
			subtitle: string
	  }
	| {
			id: "typed-path"
			kind: "typed"
			path: string
			label: string
			subtitle: string
	  }
	| {
			id: "browse-finder"
			kind: "browse"
			label: string
			subtitle: string
	  }

interface OpenProjectModalProps {
	open: boolean
	busy: boolean
	recentProjects: RecentProject[]
	currentProjectPath: string | null
	onClose: () => void
	onOpenPath: (path: string) => void | Promise<void>
	onBrowseFinder: () => Promise<string | null>
}

export function OpenProjectModal({
	open,
	busy,
	recentProjects,
	currentProjectPath,
	onClose,
	onOpenPath,
	onBrowseFinder,
}: OpenProjectModalProps) {
	const [query, setQuery] = useState("")
	const [index, setIndex] = useState(0)
	const [browsing, setBrowsing] = useState(false)
	const [fsCompletions, setFsCompletions] = useState<DirectoryCompletion[]>(
		[],
	)
	const [completing, setCompleting] = useState(false)
	const inputRef = useRef<HTMLInputElement>(null)
	const requestIdRef = useRef(0)

	const rows = useMemo((): PaletteRow[] => {
		const q = query.trim().toLowerCase()
		const pathMode = isPathQuery(query)

		const fsRows: PaletteRow[] = fsCompletions.map((item) => ({
			id: `fs:${item.path}`,
			kind: "fs" as const,
			path: item.path,
			label: item.name,
			subtitle: item.path,
		}))

		const recentRows: PaletteRow[] = recentProjects
			.filter((project) => {
				if (pathMode && q) {
					// In path mode, only keep recents that still match the typed path.
					return (
						project.path.toLowerCase().includes(q) ||
						projectLabel(project.path).toLowerCase().includes(q)
					)
				}
				if (!q) return true
				const label = projectLabel(project.path).toLowerCase()
				return label.includes(q) || project.path.toLowerCase().includes(q)
			})
			.filter(
				(project) =>
					!fsCompletions.some((item) => item.path === project.path),
			)
			.map((project) => ({
				id: `recent:${project.path}`,
				kind: "recent" as const,
				path: project.path,
				label: projectLabel(project.path),
				subtitle: project.path,
				isCurrent: project.path === currentProjectPath,
			}))

		const items: PaletteRow[] = []

		const trimmed = query.trim()
		if (trimmed.length > 0) {
			const alreadyListed =
				fsCompletions.some((item) => item.path === trimmed) ||
				recentProjects.some((project) => project.path === trimmed)
			if (!alreadyListed) {
				items.push({
					id: "typed-path",
					kind: "typed",
					path: trimmed,
					label: "Open path",
					subtitle: trimmed,
				})
			}
		}

		// Filesystem matches first when path-like or when name search returned hits.
		// Otherwise recents stay on top for plain search.
		if (pathMode || fsRows.length > 0) {
			items.push(...fsRows, ...recentRows)
		} else {
			items.push(...recentRows)
		}

		items.push({
			id: "browse-finder",
			kind: "browse",
			label: "Browse with Finder…",
			subtitle: "Choose a folder on disk",
		})

		return items
	}, [query, recentProjects, currentProjectPath, fsCompletions])

	useEffect(() => {
		if (!open) {
			setQuery("")
			setIndex(0)
			setBrowsing(false)
			setFsCompletions([])
			setCompleting(false)
			return
		}
		const id = window.setTimeout(() => inputRef.current?.focus(), 0)
		return () => window.clearTimeout(id)
	}, [open])

	// Debounced filesystem completion:
	// - "/Users/…" path prefix completion
	// - free text like "Volumes" / "circulo" searches common folders by name
	useEffect(() => {
		if (!open) return

		const trimmed = query.trim()
		if (!trimmed) {
			setFsCompletions([])
			setCompleting(false)
			return
		}

		const requestId = ++requestIdRef.current
		setCompleting(true)
		const timer = window.setTimeout(() => {
			void completeDirectoryPath(trimmed)
				.then((results) => {
					if (requestId !== requestIdRef.current) return
					setFsCompletions(results)
				})
				.catch(() => {
					if (requestId !== requestIdRef.current) return
					setFsCompletions([])
				})
				.finally(() => {
					if (requestId !== requestIdRef.current) return
					setCompleting(false)
				})
		}, 120)

		return () => {
			window.clearTimeout(timer)
		}
	}, [query, open])

	useEffect(() => {
		setIndex(0)
	}, [query, fsCompletions])

	useEffect(() => {
		if (index >= rows.length) {
			setIndex(Math.max(0, rows.length - 1))
		}
	}, [index, rows.length])

	if (!open) return null

	const locked = busy || browsing

	function fillPath(path: string) {
		// Trailing slash so the next keystroke continues into that folder.
		const next = path === "/" ? "/" : `${path}/`
		setQuery(next)
		requestAnimationFrame(() => {
			const el = inputRef.current
			if (!el) return
			el.focus()
			const end = next.length
			el.setSelectionRange(end, end)
		})
	}

	async function run(row: PaletteRow) {
		if (locked) return
		if (row.kind === "browse") {
			setBrowsing(true)
			try {
				const path = await onBrowseFinder()
				if (!path) return
				setQuery(path)
				await onOpenPath(path)
			} finally {
				setBrowsing(false)
			}
			return
		}
		if (row.kind === "fs") {
			// Enter opens; Tab fills (handled in keydown). Click opens.
			await onOpenPath(row.path)
			return
		}
		await onOpenPath(row.path)
	}

	return (
		<div
			className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 px-4 pt-[18vh]"
			onClick={() => {
				if (!browsing) onClose()
			}}
		>
			<div
				role="dialog"
				aria-modal="true"
				aria-label="Open project"
				className="frosted-strong w-full max-w-md overflow-hidden rounded-lg border border-border shadow-2xl"
				onClick={(event) => event.stopPropagation()}
			>
				<input
					ref={inputRef}
					value={query}
					onChange={(event) => setQuery(event.target.value)}
					disabled={locked}
					onKeyDown={(event) => {
						if (event.key === "Escape") {
							event.preventDefault()
							if (!browsing) onClose()
							return
						}
						if (event.key === "Tab") {
							const row = rows[index]
							if (row?.kind === "fs") {
								event.preventDefault()
								fillPath(row.path)
								return
							}
							// Common prefix when a single FS match is obvious.
							if (fsCompletions.length === 1) {
								event.preventDefault()
								fillPath(fsCompletions[0].path)
								return
							}
							if (fsCompletions.length > 1) {
								event.preventDefault()
								fillPath(fsCompletions[0].path)
							}
							return
						}
						if (event.key === "ArrowDown") {
							event.preventDefault()
							setIndex((current) =>
								rows.length > 0 ? (current + 1) % rows.length : 0,
							)
							return
						}
						if (event.key === "ArrowUp") {
							event.preventDefault()
							setIndex((current) =>
								rows.length > 0
									? (current - 1 + rows.length) % rows.length
									: 0,
							)
							return
						}
						if (event.key === "Enter" && rows[index]) {
							event.preventDefault()
							void run(rows[index])
						}
					}}
					placeholder="Search folders (Volumes, Desktop…) or type a path…"
					className="w-full border-b border-border bg-transparent px-4 py-3 font-mono text-sm text-fg outline-none placeholder:font-sans placeholder:text-muted disabled:opacity-50"
					spellCheck={false}
					autoComplete="off"
				/>

				<ul className="max-h-72 overflow-y-auto py-1">
					{rows.length === 0 ? (
						<li className="px-4 py-3 text-sm text-muted">No matches</li>
					) : (
						rows.map((row, rowIndex) => {
							const active = rowIndex === index
							const Icon =
								row.kind === "browse"
									? FolderSearch
									: row.kind === "typed"
										? FolderOpen
										: row.kind === "fs"
											? HardDrive
											: Folder

							return (
								<li key={row.id}>
									<button
										type="button"
										disabled={locked}
										onMouseEnter={() => setIndex(rowIndex)}
										onClick={() => void run(row)}
										className={cn(
											"flex w-full items-start gap-3 px-4 py-2 text-left transition disabled:opacity-50",
											active
												? "bg-white/10 text-fg"
												: "text-fg/90 hover:bg-white/5",
										)}
									>
										<Icon
											className={cn(
												"mt-0.5 size-4 shrink-0",
												active ? "text-fg" : "text-muted",
											)}
										/>
										<span className="min-w-0 flex-1">
											<span className="flex items-center gap-2 text-sm">
												<span className="truncate">{row.label}</span>
												{row.kind === "recent" && row.isCurrent ? (
													<span className="shrink-0 text-[10px] uppercase tracking-wide text-muted">
														current
													</span>
												) : null}
												{row.kind === "fs" ? (
													<span className="shrink-0 text-[10px] uppercase tracking-wide text-muted">
														folder
													</span>
												) : null}
											</span>
											<span className="mt-0.5 block truncate font-mono text-[11px] text-muted">
												{row.subtitle}
											</span>
										</span>
										{row.kind === "browse" ? (
											<span className="shrink-0 self-center text-[11px] text-muted">
												Finder
											</span>
										) : row.kind === "fs" ? (
											<span className="shrink-0 self-center text-[11px] text-muted">
												tab
											</span>
										) : null}
									</button>
								</li>
							)
						})
					)}
				</ul>

				<div className="flex items-center justify-between border-t border-border px-4 py-2 text-[11px] text-muted">
					<span>↑↓ · ↵ open · tab fill · esc</span>
					<span>
						{browsing
							? "Opening Finder…"
							: completing
								? "Searching folders…"
								: query.trim()
									? `${fsCompletions.length} folder${fsCompletions.length === 1 ? "" : "s"}`
									: "Open project"}
					</span>
				</div>
			</div>
		</div>
	)
}
