import { ChevronDown, Pencil } from "lucide-react"
import { useEffect, useRef, useState } from "react"
import { useDismissOnOutside } from "@/hooks/use-dismiss-on-outside"
import { cn } from "@/lib/utils"

const MAX_TITLE_LENGTH = 120

interface SessionTitleMenuProps {
	sessionId: string
	title: string
	onRename: (id: string, title: string) => Promise<void>
}

export function SessionTitleMenu({ sessionId, title, onRename }: SessionTitleMenuProps) {
	const [open, setOpen] = useState(false)
	const [renaming, setRenaming] = useState(false)
	const [draft, setDraft] = useState(title)
	const [pending, setPending] = useState(false)
	const rootRef = useRef<HTMLDivElement>(null)
	const inputRef = useRef<HTMLInputElement>(null)

	useDismissOnOutside(rootRef, () => {
		setOpen(false)
		setRenaming(false)
		setDraft(title)
	}, open)

	useEffect(() => {
		if (renaming) {
			inputRef.current?.focus()
			inputRef.current?.select()
		}
	}, [renaming])

	useEffect(() => {
		if (!open) {
			setRenaming(false)
			setDraft(title)
		}
	}, [open, title])

	async function handleSave() {
		const trimmed = draft.trim()
		if (!trimmed || trimmed === title.trim()) {
			setRenaming(false)
			setOpen(false)
			setDraft(title)
			return
		}

		setPending(true)
		try {
			await onRename(sessionId, trimmed)
			setOpen(false)
			setRenaming(false)
		} finally {
			setPending(false)
		}
	}

	return (
		<div ref={rootRef} className="pointer-events-auto relative shrink-0">
			<button
				type="button"
				title="Opciones de sesión"
				onClick={() => setOpen((value) => !value)}
				className={cn(
					"flex size-5 items-center justify-center rounded-md text-muted-foreground/60 transition-colors hover:bg-accent hover:text-foreground",
					open && "bg-accent text-foreground",
				)}
			>
				<ChevronDown className="size-3" />
			</button>

			{open ? (
				<div className="absolute left-0 top-full z-50 mt-1 min-w-44 overflow-hidden rounded-lg border border-border bg-popover py-1 shadow-lg">
					{renaming ? (
						<div className="space-y-1 px-2 py-1.5">
							<input
								ref={inputRef}
								type="text"
								value={draft}
								maxLength={MAX_TITLE_LENGTH}
								disabled={pending}
								onChange={(event) => setDraft(event.target.value)}
								onKeyDown={(event) => {
									if (event.key === "Enter") {
										event.preventDefault()
										void handleSave()
									}
									if (event.key === "Escape") {
										event.preventDefault()
										setRenaming(false)
										setDraft(title)
									}
								}}
								className="h-7 w-full rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-ring"
								placeholder="Nombre de la sesión"
							/>
							<div className="flex justify-end gap-1">
								<button
									type="button"
									disabled={pending}
									onClick={() => {
										setRenaming(false)
										setDraft(title)
									}}
									className="h-6 rounded-md px-2 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground disabled:opacity-50"
								>
									Cancelar
								</button>
								<button
									type="button"
									disabled={pending || !draft.trim()}
									onClick={() => void handleSave()}
									className="h-6 rounded-md bg-primary px-2 text-xs text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
								>
									Guardar
								</button>
							</div>
						</div>
					) : (
						<button
							type="button"
							className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs transition-colors hover:bg-white/10"
							onClick={() => setRenaming(true)}
						>
							<Pencil className="size-3.5 shrink-0 text-muted-foreground" />
							Cambiar nombre
						</button>
					)}
				</div>
			) : null}
		</div>
	)
}