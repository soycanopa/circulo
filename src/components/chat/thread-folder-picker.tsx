import { open } from "@tauri-apps/plugin-dialog"
import { ChevronDown, FolderOpen, Search } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import { createPortal } from "react-dom"
import { SelectorMenuItem } from "@/components/chat/selector-menu-item"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { getProjectDirectoryLabel } from "@/lib/project-display"
import {
	filterRecentProjects,
	getRecentProjectLabel,
	getRecentProjects,
	MAX_RECENT_PROJECTS_DISPLAY,
} from "@/lib/recent-projects"
import { cn } from "@/lib/utils"

interface ThreadFolderPickerProps {
	projectPath: string | null
	onOpenProject: (path: string) => Promise<void>
	onClose: () => void
}

export function ThreadFolderPicker({
	projectPath,
	onOpenProject,
	onClose,
}: ThreadFolderPickerProps) {
	const [openMenu, setOpenMenu] = useState(false)
	const [query, setQuery] = useState("")
	const [pending, setPending] = useState(false)
	const [menuPosition, setMenuPosition] = useState<{ top: number; left: number } | null>(null)
	const triggerRef = useRef<HTMLButtonElement>(null)
	const menuRef = useRef<HTMLDivElement>(null)

	const allRecent = useMemo(() => getRecentProjects(), [openMenu])
	const showSearch = allRecent.length > MAX_RECENT_PROJECTS_DISPLAY
	const visibleRecent = useMemo(() => {
		const filtered = filterRecentProjects(allRecent, query)
		return filtered.slice(0, MAX_RECENT_PROJECTS_DISPLAY)
	}, [allRecent, query])

	const folderLabel = projectPath ? getProjectDirectoryLabel(projectPath) : "Seleccionar carpeta"
	const noDragProps = windowNoDragProps()

	function updateMenuPosition() {
		const rect = triggerRef.current?.getBoundingClientRect()
		if (!rect) return
		setMenuPosition({
			top: rect.top - 6,
			left: rect.left,
		})
	}

	function closeMenu() {
		setOpenMenu(false)
		setQuery("")
		setMenuPosition(null)
	}

	useEffect(() => {
		if (!openMenu) return

		function handlePointerDown(event: PointerEvent) {
			const target = event.target
			if (!(target instanceof Node)) return
			if (triggerRef.current?.contains(target)) return
			if (menuRef.current?.contains(target)) return
			closeMenu()
		}

		document.addEventListener("pointerdown", handlePointerDown, true)
		return () => document.removeEventListener("pointerdown", handlePointerDown, true)
	}, [openMenu])

	useEffect(() => {
		if (!openMenu) return
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
	}, [openMenu])

	async function handlePickProject(path: string) {
		setPending(true)
		try {
			await onOpenProject(path)
			closeMenu()
			onClose()
		} finally {
			setPending(false)
		}
	}

	async function handleOpenFolder() {
		const selected = await open({ directory: true, multiple: false, title: "Abrir carpeta" })
		if (!selected || Array.isArray(selected)) return
		await handlePickProject(selected)
	}

	return (
		<>
			<button
				ref={triggerRef}
				type="button"
				disabled={pending}
				{...noDragProps}
				onMouseDown={(event) => event.preventDefault()}
				onClick={(event) => {
					event.preventDefault()
					event.stopPropagation()
					if (openMenu) {
						closeMenu()
						return
					}
					updateMenuPosition()
					setOpenMenu(true)
				}}
				className={cn(
					"inline-flex max-w-[11rem] min-w-0 shrink-0 items-center gap-1.5 text-xs text-foreground/90 transition-colors hover:text-foreground disabled:opacity-60",
					openMenu && "text-foreground",
				)}
			>
				<FolderOpen className="size-3.5 shrink-0 text-muted-foreground" />
				<span className="min-w-0 truncate font-medium">{folderLabel}</span>
				<ChevronDown className="size-3 shrink-0 text-muted-foreground" />
			</button>

			{openMenu && menuPosition
				? createPortal(
						<div
							ref={menuRef}
							{...noDragProps}
							className="fixed z-[200] w-56 -translate-y-full overflow-hidden rounded-lg border border-popover-border bg-popover shadow-lg"
							style={{ top: menuPosition.top, left: menuPosition.left }}
						>
							<div className="p-1">
								<SelectorMenuItem onClick={() => void handleOpenFolder()}>
									<span className="inline-flex items-center gap-2">
										<FolderOpen className="size-3.5 shrink-0 text-muted-foreground" />
										Open Folder
									</span>
								</SelectorMenuItem>
							</div>

							{visibleRecent.length > 0 ? (
								<>
									<div className="border-t border-border/50 px-2.5 py-1.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
										Recientes
									</div>
									{showSearch ? (
										<div className="border-b border-border/50 px-2 py-1.5">
											<div className="flex items-center gap-1.5 rounded-md border border-border/60 bg-background px-2 py-1">
												<Search className="size-3 shrink-0 text-muted-foreground" />
												<input
													type="text"
													value={query}
													onChange={(event) => setQuery(event.target.value)}
													placeholder="Buscar carpeta…"
													{...noDragProps}
													className="min-w-0 flex-1 bg-transparent text-xs outline-none placeholder:text-muted-foreground"
												/>
											</div>
										</div>
									) : null}
									<div className="max-h-40 overflow-y-auto p-1">
										{visibleRecent.map((path) => (
											<SelectorMenuItem
												key={path}
												active={path === projectPath}
												onClick={() => void handlePickProject(path)}
											>
												<span className="truncate">{getRecentProjectLabel(path)}</span>
											</SelectorMenuItem>
										))}
									</div>
								</>
							) : null}
						</div>,
						document.body,
					)
				: null}
		</>
	)
}