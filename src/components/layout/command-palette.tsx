import { useEffect, useRef, useState } from "react"
import { cn } from "@/lib/utils"

export interface CommandPaletteItem {
	id: string
	label: string
	shortcut?: string
	onSelect: () => void
}

interface CommandPaletteProps {
	open: boolean
	items: CommandPaletteItem[]
	onClose: () => void
}

export function CommandPalette({ open, items, onClose }: CommandPaletteProps) {
	const [query, setQuery] = useState("")
	const [index, setIndex] = useState(0)
	const inputRef = useRef<HTMLInputElement>(null)

	const filtered = items.filter((item) =>
		item.label.toLowerCase().includes(query.trim().toLowerCase()),
	)

	useEffect(() => {
		if (!open) {
			setQuery("")
			setIndex(0)
			return
		}
		inputRef.current?.focus()
	}, [open])

	useEffect(() => {
		setIndex(0)
	}, [query])

	if (!open) return null

	function run(item: CommandPaletteItem) {
		item.onSelect()
		onClose()
	}

	return (
		<div
			className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 px-4 pt-[18vh]"
			onClick={onClose}
		>
			<div
				className="frosted-strong w-full max-w-md overflow-hidden rounded-lg border border-border shadow-2xl"
				onClick={(event) => event.stopPropagation()}
			>
				<input
					ref={inputRef}
					value={query}
					onChange={(event) => setQuery(event.target.value)}
					onKeyDown={(event) => {
						if (event.key === "Escape") {
							event.preventDefault()
							onClose()
							return
						}
						if (event.key === "ArrowDown") {
							event.preventDefault()
							setIndex((current) =>
								filtered.length > 0 ? (current + 1) % filtered.length : 0,
							)
							return
						}
						if (event.key === "ArrowUp") {
							event.preventDefault()
							setIndex((current) =>
								filtered.length > 0
									? (current - 1 + filtered.length) % filtered.length
									: 0,
							)
							return
						}
						if (event.key === "Enter" && filtered[index]) {
							event.preventDefault()
							run(filtered[index])
						}
					}}
					placeholder="Type a command…"
					className="w-full border-b border-border bg-transparent px-4 py-3 text-sm text-fg outline-none placeholder:text-muted"
				/>
				<ul className="max-h-64 overflow-y-auto py-1">
					{filtered.length === 0 ? (
						<li className="px-4 py-3 text-sm text-muted">No commands found</li>
					) : (
						filtered.map((item, itemIndex) => (
							<li key={item.id}>
								<button
									type="button"
									onClick={() => run(item)}
									className={cn(
										"flex w-full items-center justify-between px-4 py-2 text-left text-sm transition",
										itemIndex === index
											? "bg-white/10 text-fg"
											: "text-fg/90 hover:bg-white/5",
									)}
								>
									<span>{item.label}</span>
									{item.shortcut ? (
										<span className="text-xs text-muted">{item.shortcut}</span>
									) : null}
								</button>
							</li>
						))
					)}
				</ul>
			</div>
		</div>
	)
}
