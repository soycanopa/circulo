import { MoreHorizontal, Pencil, Trash2 } from "lucide-react"
import { useEffect, useRef, useState } from "react"
import { useDismissOnOutside } from "@/hooks/use-dismiss-on-outside"
import { cn } from "@/lib/utils"

interface ProjectActionsMenuProps {
	label: string
	onRename: (alias: string) => void
	onDelete: () => void
	className?: string
}

export function ProjectActionsMenu({
	label,
	onRename,
	onDelete,
	className,
}: ProjectActionsMenuProps) {
	const [open, setOpen] = useState(false)
	const [renaming, setRenaming] = useState(false)
	const [draft, setDraft] = useState(label)
	const rootRef = useRef<HTMLDivElement>(null)
	const inputRef = useRef<HTMLInputElement>(null)

	useDismissOnOutside(rootRef, () => {
		setOpen(false)
		setRenaming(false)
		setDraft(label)
	}, open)

	useEffect(() => {
		if (!open) {
			setRenaming(false)
			setDraft(label)
		}
	}, [open, label])

	useEffect(() => {
		if (renaming) {
			inputRef.current?.focus()
			inputRef.current?.select()
		}
	}, [renaming])

	function submitRename() {
		const trimmed = draft.trim()
		if (trimmed) onRename(trimmed)
		setOpen(false)
		setRenaming(false)
	}

	return (
		<div ref={rootRef} className={cn("absolute right-8 top-1/2 z-10 -translate-y-1/2", className)}>
			<button
				type="button"
				title="Opciones del proyecto"
				onClick={(event) => {
					event.stopPropagation()
					setOpen((value) => !value)
				}}
				className={cn(
					"flex size-6 items-center justify-center rounded-md text-sidebar-foreground/50 opacity-0 transition-opacity hover:bg-sidebar-accent hover:text-sidebar-accent-foreground group-hover/menu-item:opacity-100",
					open && "bg-sidebar-accent text-sidebar-accent-foreground opacity-100",
				)}
			>
				<MoreHorizontal className="size-3.5" />
			</button>

			{open ? (
				<div className="absolute right-0 top-full z-20 mt-1 min-w-40 overflow-hidden rounded-lg border border-border bg-popover py-1 shadow-lg">
					{renaming ? (
						<div className="space-y-1 px-2 py-1.5">
							<input
								ref={inputRef}
								type="text"
								value={draft}
								onChange={(event) => setDraft(event.target.value)}
								onKeyDown={(event) => {
									if (event.key === "Enter") {
										event.preventDefault()
										submitRename()
									}
									if (event.key === "Escape") {
										event.preventDefault()
										setRenaming(false)
										setDraft(label)
									}
								}}
								className="h-7 w-full rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-ring"
							/>
							<div className="flex justify-end gap-1">
								<button
									type="button"
									className="rounded px-2 py-0.5 text-[10px] text-muted-foreground hover:bg-white/10"
									onClick={(event) => {
										event.stopPropagation()
										setRenaming(false)
										setDraft(label)
									}}
								>
									Cancelar
								</button>
								<button
									type="button"
									className="rounded bg-primary px-2 py-0.5 text-[10px] text-primary-foreground"
									onClick={(event) => {
										event.stopPropagation()
										submitRename()
									}}
								>
									Guardar
								</button>
							</div>
						</div>
					) : (
						<>
							<button
								type="button"
								className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs transition-colors hover:bg-white/10"
								onClick={(event) => {
									event.stopPropagation()
									setRenaming(true)
								}}
							>
								<Pencil className="size-3.5 shrink-0 text-muted-foreground" />
								Renombrar
							</button>
							<button
								type="button"
								className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs text-destructive transition-colors hover:bg-destructive/10"
								onClick={(event) => {
									event.stopPropagation()
									setOpen(false)
									onDelete()
								}}
							>
								<Trash2 className="size-3.5 shrink-0" />
								Eliminar proyecto
							</button>
						</>
					)}
				</div>
			) : null}
		</div>
	)
}