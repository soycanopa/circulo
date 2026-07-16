import { ChevronDown, Pencil } from "lucide-react"
import { useEffect, useRef, useState } from "react"
import { createPortal } from "react-dom"
import { windowNoDragProps } from "@/hooks/use-window-drag"
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
	const [menuPosition, setMenuPosition] = useState<{ top: number; left: number } | null>(null)
	const triggerRef = useRef<HTMLButtonElement>(null)
	const menuRef = useRef<HTMLDivElement>(null)
	const inputRef = useRef<HTMLInputElement>(null)

	function closeMenu() {
		setOpen(false)
		setRenaming(false)
		setDraft(title)
		setMenuPosition(null)
	}

	function updateMenuPosition() {
		const rect = triggerRef.current?.getBoundingClientRect()
		if (!rect) return
		setMenuPosition({ top: rect.bottom + 4, left: rect.left })
	}

	useEffect(() => {
		if (!open) return

		function handlePointerDown(event: PointerEvent) {
			const target = event.target
			if (!(target instanceof Node)) return
			if (triggerRef.current?.contains(target)) return
			if (menuRef.current?.contains(target)) return
			closeMenu()
		}

		document.addEventListener("pointerdown", handlePointerDown, true)
		return () => document.removeEventListener("pointerdown", handlePointerDown, true)
	}, [open, title])

	useEffect(() => {
		if (!open) return
		updateMenuPosition()

		function handleLayoutChange() {
			updateMenuPosition()
		}

		window.addEventListener("resize", handleLayoutChange)
		window.addEventListener("scroll", handleLayoutChange, true)
		return () => {
			window.removeEventListener("resize", handleLayoutChange)
			window.removeEventListener("scroll", handleLayoutChange, true)
		}
	}, [open])

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
			closeMenu()
			return
		}

		setPending(true)
		try {
			await onRename(sessionId, trimmed)
			closeMenu()
		} finally {
			setPending(false)
		}
	}

	const noDragProps = windowNoDragProps()

	return (
		<>
			<button
				ref={triggerRef}
				type="button"
				title="Opciones de sesión"
				{...noDragProps}
				onMouseDown={(event) => event.preventDefault()}
				onClick={(event) => {
					event.preventDefault()
					event.stopPropagation()
					if (open) {
						closeMenu()
						return
					}
					updateMenuPosition()
					setOpen(true)
				}}
				className={cn(
					"flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground/60 transition-colors hover:bg-accent hover:text-foreground",
					open && "bg-accent text-foreground",
				)}
			>
				<ChevronDown className="size-3.5" />
			</button>

			{open && menuPosition
				? createPortal(
						<div
							ref={menuRef}
							{...noDragProps}
							className="fixed z-[200] min-w-44 overflow-hidden rounded-lg border border-popover-border bg-popover py-1 shadow-lg"
							style={{ top: menuPosition.top, left: menuPosition.left }}
						>
							{renaming ? (
								<div className="space-y-1 px-2 py-1.5">
									<input
										ref={inputRef}
										type="text"
										value={draft}
										maxLength={MAX_TITLE_LENGTH}
										disabled={pending}
										{...noDragProps}
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
											{...noDragProps}
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
											{...noDragProps}
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
									{...noDragProps}
									className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs transition-colors hover:bg-white/10"
									onClick={() => setRenaming(true)}
								>
									<Pencil className="size-3.5 shrink-0 text-muted-foreground" />
									Cambiar nombre
								</button>
							)}
						</div>,
						document.body,
					)
				: null}
		</>
	)
}