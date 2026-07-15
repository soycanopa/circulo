import { Archive, MoreHorizontal, Trash2 } from "lucide-react"
import { useRef, useState } from "react"
import { useDismissOnOutside } from "@/hooks/use-dismiss-on-outside"
import { cn } from "@/lib/utils"

interface SessionActionsMenuProps {
	onArchive: () => void
	onDelete: () => void
	className?: string
}

export function SessionActionsMenu({ onArchive, onDelete, className }: SessionActionsMenuProps) {
	const [open, setOpen] = useState(false)
	const rootRef = useRef<HTMLDivElement>(null)

	useDismissOnOutside(rootRef, () => setOpen(false), open)

	return (
		<div ref={rootRef} className={cn("absolute right-1 top-1/2 z-10 -translate-y-1/2", className)}>
			<button
				type="button"
				title="Opciones de sesión"
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
				<div className="absolute right-0 top-full z-20 mt-1 min-w-36 overflow-hidden rounded-lg border border-border bg-popover py-1 shadow-lg">
					<button
						type="button"
						className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs transition-colors hover:bg-white/10"
						onClick={(event) => {
							event.stopPropagation()
							setOpen(false)
							onArchive()
						}}
					>
						<Archive className="size-3.5 shrink-0 text-muted-foreground" />
						Archivar
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
						Eliminar
					</button>
				</div>
			) : null}
		</div>
	)
}