import {
	ChevronDown,
	ChevronRight,
	FolderOpen,
	MessageSquare,
	MessageSquarePlus,
	Pencil,
	Plus,
	Settings,
	Trash2,
} from "lucide-react"
import {
	useEffect,
	useMemo,
	useRef,
	useState,
	type MouseEvent as ReactMouseEvent,
	type PointerEvent as ReactPointerEvent,
} from "react"
import { WindowChromeControls } from "@/components/layout/window-chrome-controls"
import { AgentStatusIndicator } from "@/components/layout/agent-status-indicator"
import { isGeneralChatsPath, projectName } from "@/lib/workspace"
import { cn } from "@/lib/utils"
import type { AgentRuntimeState } from "@/lib/agent-registry"
import type { ChatSessionSummary, WorkspaceEntry } from "@/types/acp"

interface AppSidebarProps {
	sessionId: string | null
	historyViewSessionId: string | null
	agentRuntimes: AgentRuntimeState[]
	busy: boolean
	/** Absolute path for general chats of the active workspace. */
	generalChatsPath: string | null
	generalChats: ChatSessionSummary[]
	/** Nested chats keyed by absolute project path. */
	projectChatsByPath: Record<string, ChatSessionSummary[]>
	workspaces: WorkspaceEntry[]
	activeWorkspaceId: string | null
	currentProjectPath: string | null
	onNewChat: () => void
	onOpenProject: () => void
	onOpenSettings: () => void
	onOpenChat: (sessionId: string, ownerPath: string) => void
	onRenameChat: (
		sessionId: string,
		ownerPath: string,
		title: string,
	) => void
	onDeleteChat: (sessionId: string, ownerPath: string) => void
	onSelectProject: (path: string) => void
	onSelectGeneralChats?: () => void
	onAddWorkspace: () => void
	onSelectWorkspace: (workspaceId: string) => void
	/** Remove a space (drag dot out). Cannot remove the last one. */
	onDeleteWorkspace: (workspaceId: string) => void
	onHideSidebar: () => void
}

export function AppSidebar({
	sessionId,
	historyViewSessionId,
	agentRuntimes,
	busy,
	generalChatsPath,
	generalChats,
	projectChatsByPath,
	workspaces,
	activeWorkspaceId,
	currentProjectPath,
	onNewChat,
	onOpenProject,
	onOpenSettings,
	onOpenChat,
	onRenameChat,
	onDeleteChat,
	onSelectProject,
	onSelectGeneralChats,
	onAddWorkspace,
	onSelectWorkspace,
	onDeleteWorkspace,
	onHideSidebar,
}: AppSidebarProps) {
	const generalActive = isGeneralChatsPath(currentProjectPath)

	const activeWorkspace = useMemo(
		() => workspaces.find((w) => w.id === activeWorkspaceId) ?? null,
		[workspaces, activeWorkspaceId],
	)

	/** Strict isolation: only paths listed on the active workspace. */
	const projectPaths = useMemo(() => {
		const paths: string[] = []
		const seen = new Set<string>()
		for (const path of activeWorkspace?.projectPaths ?? []) {
			if (isGeneralChatsPath(path) || seen.has(path)) continue
			seen.add(path)
			paths.push(path)
		}
		return paths
	}, [activeWorkspace])

	const [expanded, setExpanded] = useState<Record<string, boolean>>({})
	const [editingKey, setEditingKey] = useState<string | null>(null)
	const [draftTitle, setDraftTitle] = useState("")
	const [draggingId, setDraggingId] = useState<string | null>(null)
	const [dragPos, setDragPos] = useState<{ x: number; y: number } | null>(null)
	const [willDelete, setWillDelete] = useState(false)
	const [contextMenu, setContextMenu] = useState<{
		id: string
		x: number
		y: number
	} | null>(null)
	const dragOrigin = useRef<{ x: number; y: number; id: string } | null>(null)
	const dotsBarRef = useRef<HTMLDivElement>(null)
	const movedRef = useRef(false)
	const willDeleteRef = useRef(false)

	useEffect(() => {
		if (!currentProjectPath || isGeneralChatsPath(currentProjectPath)) return
		setExpanded((prev) =>
			prev[currentProjectPath] ? prev : { ...prev, [currentProjectPath]: true },
		)
	}, [currentProjectPath])

	useEffect(() => {
		if (!contextMenu) return
		function close() {
			setContextMenu(null)
		}
		function onKey(event: KeyboardEvent) {
			if (event.key === "Escape") close()
		}
		window.addEventListener("pointerdown", close)
		window.addEventListener("keydown", onKey)
		return () => {
			window.removeEventListener("pointerdown", close)
			window.removeEventListener("keydown", onKey)
		}
	}, [contextMenu])

	// Escape cancels an in-progress workspace-dot drag (no switch, no delete).
	useEffect(() => {
		if (!draggingId) return
		function onKey(event: KeyboardEvent) {
			if (event.key !== "Escape") return
			event.preventDefault()
			dragOrigin.current = null
			movedRef.current = false
			willDeleteRef.current = false
			setDraggingId(null)
			setDragPos(null)
			setWillDelete(false)
		}
		window.addEventListener("keydown", onKey)
		return () => window.removeEventListener("keydown", onKey)
	}, [draggingId])

	function startRename(ownerPath: string, chat: ChatSessionSummary) {
		setEditingKey(`${ownerPath}::${chat.sessionId}`)
		setDraftTitle(chat.title)
	}

	function commitRename(ownerPath: string, sessionId: string) {
		const title = draftTitle.trim()
		setEditingKey(null)
		if (!title) return
		onRenameChat(sessionId, ownerPath, title)
	}

	function toggleProject(path: string) {
		setExpanded((prev) => ({ ...prev, [path]: !prev[path] }))
	}

	function onDotPointerDown(
		event: ReactPointerEvent<HTMLButtonElement>,
		workspaceId: string,
	) {
		if (busy || event.button !== 0) return
		setContextMenu(null)
		event.currentTarget.setPointerCapture(event.pointerId)
		dragOrigin.current = { x: event.clientX, y: event.clientY, id: workspaceId }
		movedRef.current = false
		willDeleteRef.current = false
		setDraggingId(workspaceId)
		setDragPos({ x: event.clientX, y: event.clientY })
		setWillDelete(false)
	}

	function onDotPointerMove(event: ReactPointerEvent<HTMLButtonElement>) {
		const origin = dragOrigin.current
		if (!origin || origin.id !== draggingId) return

		setDragPos({ x: event.clientX, y: event.clientY })

		const dx = event.clientX - origin.x
		const dy = event.clientY - origin.y
		if (Math.hypot(dx, dy) > 4) movedRef.current = true

		const bar = dotsBarRef.current
		if (!bar) return
		const rect = bar.getBoundingClientRect()
		const outside =
			event.clientX < rect.left - 10 ||
			event.clientX > rect.right + 10 ||
			event.clientY < rect.top - 14 ||
			event.clientY > rect.bottom + 14
		const del = outside && movedRef.current && workspaces.length > 1
		willDeleteRef.current = del
		setWillDelete(del)
	}

	function onDotPointerUp(
		event: ReactPointerEvent<HTMLButtonElement>,
		workspaceId: string,
	) {
		try {
			event.currentTarget.releasePointerCapture(event.pointerId)
		} catch {
			// ignore
		}

		// Drag already cancelled (e.g. Escape) — ignore this pointer up.
		if (!dragOrigin.current || dragOrigin.current.id !== workspaceId) {
			return
		}

		const shouldDelete = willDeleteRef.current && movedRef.current
		const wasTap = !movedRef.current

		dragOrigin.current = null
		willDeleteRef.current = false
		setDraggingId(null)
		setDragPos(null)
		setWillDelete(false)

		if (shouldDelete && workspaces.length > 1) {
			onDeleteWorkspace(workspaceId)
			return
		}
		if (wasTap) {
			onSelectWorkspace(workspaceId)
		}
	}

	function onDotContextMenu(
		event: ReactMouseEvent,
		workspaceId: string,
	) {
		event.preventDefault()
		event.stopPropagation()
		if (busy || workspaces.length <= 1) return
		setContextMenu({ id: workspaceId, x: event.clientX, y: event.clientY })
	}

	function renderChatRow(ownerPath: string, chat: ChatSessionSummary) {
		const active =
			currentProjectPath === ownerPath &&
			(chat.sessionId === sessionId ||
				chat.sessionId === historyViewSessionId)
		const editKey = `${ownerPath}::${chat.sessionId}`
		const editing = editingKey === editKey

		return (
			<div
				key={editKey}
				className={cn(
					"group flex items-start gap-1 rounded-md transition",
					active ? "bg-white/10" : "hover:bg-white/5",
				)}
			>
				{editing ? (
					<input
						value={draftTitle}
						onChange={(event) => setDraftTitle(event.target.value)}
						onBlur={() => commitRename(ownerPath, chat.sessionId)}
						onKeyDown={(event) => {
							if (event.key === "Enter") {
								event.preventDefault()
								commitRename(ownerPath, chat.sessionId)
							}
							if (event.key === "Escape") {
								setEditingKey(null)
							}
						}}
						className="m-1 min-w-0 flex-1 rounded border border-border bg-black/20 px-2 py-1 text-xs text-fg outline-none focus:border-white/20"
						autoFocus
					/>
				) : (
					<button
						type="button"
						onClick={() => onOpenChat(chat.sessionId, ownerPath)}
						disabled={busy}
						className={cn(
							"flex min-w-0 flex-1 items-start gap-2 px-2.5 py-1.5 text-left text-xs disabled:opacity-50",
							active ? "text-fg" : "text-fg/80",
						)}
					>
						<MessageSquare className="mt-0.5 size-3.5 shrink-0 text-muted" />
						<span className="line-clamp-2">{chat.title}</span>
					</button>
				)}
				{!editing ? (
					<>
						<button
							type="button"
							onClick={() => startRename(ownerPath, chat)}
							disabled={busy}
							className="mt-1 rounded p-1 text-muted opacity-0 transition hover:bg-white/10 hover:text-fg group-hover:opacity-100 disabled:opacity-40"
							title="Rename chat"
						>
							<Pencil className="size-3.5" />
						</button>
						<button
							type="button"
							onClick={() => onDeleteChat(chat.sessionId, ownerPath)}
							disabled={busy}
							className="mr-1 mt-1 rounded p-1 text-muted opacity-0 transition hover:bg-white/10 hover:text-red-300 group-hover:opacity-100 disabled:opacity-40"
							title="Delete chat"
						>
							<Trash2 className="size-3.5" />
						</button>
					</>
				) : null}
			</div>
		)
	}

	return (
		<div className="flex h-full min-h-0 flex-col">
			<div
				className="flex h-12 shrink-0 items-center border-b border-border pb-0.5"
				data-tauri-drag-region="deep"
			>
				<WindowChromeControls
					sidebarOpen
					layout="sidebar"
					onToggleSidebar={onHideSidebar}
				/>
			</div>

			{/* Chats + Projects + workspace dots (middle zone, above ACP/settings). */}
			<div className="flex min-h-0 flex-1 flex-col">
				<div className="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto p-3 pb-2">
					<button
						type="button"
						onClick={onNewChat}
						disabled={busy}
						className="flex shrink-0 items-center gap-2 rounded-md bg-white/10 px-2.5 py-2 text-left text-sm font-medium text-fg transition hover:bg-white/15 disabled:opacity-40"
					>
						<MessageSquarePlus className="size-4 shrink-0" />
						New chat
					</button>
					<button
						type="button"
						onClick={onOpenProject}
						disabled={busy}
						className="flex shrink-0 items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm text-fg/90 transition hover:bg-white/5 disabled:opacity-50"
					>
						<FolderOpen className="size-4 shrink-0" />
						Open project
					</button>

					<div className="mt-4 flex items-center justify-between px-2.5">
						<button
							type="button"
							onClick={() => onSelectGeneralChats?.()}
							disabled={busy || !onSelectGeneralChats}
							className={cn(
								"text-[11px] uppercase tracking-wider transition",
								generalActive ? "text-fg" : "text-muted/70 hover:text-muted",
							)}
							title="Switch to general chats in this workspace"
						>
							Chats
						</button>
						{generalActive ? (
							<span className="text-[10px] text-muted">active</span>
						) : null}
					</div>
					{!generalChatsPath ? (
						<p className="px-2.5 py-1 text-xs text-muted/80">Loading chats…</p>
					) : generalChats.length === 0 ? (
						<p className="px-2.5 py-1 text-xs text-muted/80">
							No general chats in this workspace yet.
						</p>
					) : (
						<div className="flex flex-col gap-0.5">
							{generalChats.map((chat) =>
								renderChatRow(generalChatsPath, chat),
							)}
						</div>
					)}

					<div className="mt-4 px-2.5 text-[11px] uppercase tracking-wider text-muted/70">
						Projects
					</div>
					{projectPaths.length === 0 ? (
						<p className="px-2.5 py-1 text-xs text-muted/80">
							Open a project — it stays in this workspace only.
						</p>
					) : (
						<div className="flex flex-col gap-0.5">
							{projectPaths.map((path) => {
								const isActive =
									currentProjectPath === path &&
									!isGeneralChatsPath(currentProjectPath)
								const isOpen = expanded[path] ?? isActive
								const nested = projectChatsByPath[path] ?? []

								return (
									<div key={path} className="flex flex-col gap-0.5">
										<div
											className={cn(
												"group flex items-center gap-0.5 rounded-md",
												isActive ? "bg-white/10" : "hover:bg-white/5",
											)}
										>
											<button
												type="button"
												onClick={() => toggleProject(path)}
												className="rounded p-1 text-muted transition hover:text-fg"
												aria-label={
													isOpen ? "Collapse project" : "Expand project"
												}
											>
												{isOpen ? (
													<ChevronDown className="size-3.5" />
												) : (
													<ChevronRight className="size-3.5" />
												)}
											</button>
											<button
												type="button"
												onClick={() => onSelectProject(path)}
												disabled={busy}
												title={path}
												className={cn(
													"min-w-0 flex-1 truncate py-1.5 pr-2 text-left text-xs disabled:opacity-50",
													isActive ? "font-medium text-fg" : "text-fg/85",
												)}
											>
												{projectName(path)}
											</button>
										</div>
										{isOpen ? (
											<div className="ml-3 border-l border-border pl-1">
												{nested.length === 0 ? (
													<p className="px-2.5 py-1 text-[11px] text-muted/70">
														No chats in this project
													</p>
												) : (
													nested.map((chat) => renderChatRow(path, chat))
												)}
											</div>
										) : null}
									</div>
								)
							})}
						</div>
					)}
				</div>

				{/* Centered workspace dots + add — middle zone, above ACP/settings */}
				<div className="flex shrink-0 items-center justify-center px-3 py-2">
					<div
						ref={dotsBarRef}
						className="flex items-center justify-center gap-0.5"
						role="tablist"
						aria-label="Workspaces"
					>
						{workspaces.map((ws, index) => {
							const active = ws.id === activeWorkspaceId
							const dragging = draggingId === ws.id
							return (
								<button
									key={ws.id}
									type="button"
									role="tab"
									aria-selected={active}
									disabled={busy}
									title={`Workspace ${index + 1}`}
									onPointerDown={(event) => onDotPointerDown(event, ws.id)}
									onPointerMove={onDotPointerMove}
									onPointerUp={(event) => onDotPointerUp(event, ws.id)}
									onPointerCancel={() => {
										dragOrigin.current = null
										willDeleteRef.current = false
										setDraggingId(null)
										setDragPos(null)
										setWillDelete(false)
									}}
									onContextMenu={(event) => onDotContextMenu(event, ws.id)}
									className={cn(
										"flex size-5 touch-none items-center justify-center rounded-full transition disabled:opacity-40",
										"hover:bg-white/5",
										// Slot stays; ghost follows the cursor while dragging.
										dragging && "opacity-25",
									)}
								>
									<span
										className={cn(
											"block size-1.5 rounded-full transition",
											active ? "bg-fg" : "bg-muted/50",
										)}
									/>
								</button>
							)
						})}
						<button
							type="button"
							onClick={onAddWorkspace}
							disabled={busy}
							className="flex size-5 items-center justify-center rounded-full text-muted transition hover:bg-white/5 hover:text-fg disabled:opacity-40"
							title="Add workspace"
							aria-label="Add workspace"
							data-tauri-drag-region="false"
						>
							<Plus className="size-3" strokeWidth={2.25} />
						</button>
					</div>
				</div>
			</div>

			{/* Floating drag ghost — follows the mouse */}
			{draggingId && dragPos ? (
				<div
					className="pointer-events-none fixed z-[100] -translate-x-1/2 -translate-y-1/2"
					style={{ left: dragPos.x, top: dragPos.y }}
					aria-hidden
				>
					<span
						className={cn(
							"block size-2 rounded-full shadow-lg ring-2 transition-colors",
							willDelete
								? "bg-red-400 ring-red-400/40"
								: "bg-fg ring-white/20",
						)}
					/>
				</div>
			) : null}

			{/* Right-click fallback */}
			{contextMenu ? (
				<div
					role="menu"
					className="fixed z-[100] min-w-[9.5rem] overflow-hidden rounded-md border border-border bg-sidebar py-1 shadow-xl"
					style={{ left: contextMenu.x, top: contextMenu.y }}
					onPointerDown={(event) => event.stopPropagation()}
				>
					<button
						type="button"
						role="menuitem"
						className="block w-full px-3 py-1.5 text-left text-xs text-red-300 transition hover:bg-white/5"
						onClick={() => {
							const id = contextMenu.id
							setContextMenu(null)
							onDeleteWorkspace(id)
						}}
					>
						Delete workspace
					</button>
				</div>
			) : null}

			{/* ACP + Settings only */}
			<div className="shrink-0 border-t border-border px-3 py-2.5">
				<div className="flex items-center justify-between gap-2">
					<AgentStatusIndicator agents={agentRuntimes} />
					<button
						type="button"
						onClick={onOpenSettings}
						className="shrink-0 rounded p-1 text-muted transition hover:bg-white/5 hover:text-fg"
						title="Settings"
						data-tauri-drag-region="false"
					>
						<Settings className="size-4" />
					</button>
				</div>
			</div>
		</div>
	)
}
