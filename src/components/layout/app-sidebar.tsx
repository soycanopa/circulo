import {
	FolderOpen,
	MessageSquare,
	MessageSquarePlus,
	Pencil,
	Settings,
	Trash2,
} from "lucide-react"
import { useState } from "react"
import { cn } from "@/lib/utils"
import type { ChatSessionSummary, RecentProject } from "@/types/acp"

function projectLabel(path: string): string {
	if (path.includes("/.circulo/chats")) return "General Chat"
	const parts = path.split("/").filter(Boolean)
	return parts.slice(-2).join("/") || path
}

interface AppSidebarProps {
	workspaceLabel: string
	sessionId: string | null
	historyViewSessionId: string | null
	agentWarm: boolean
	progress: string | null
	busy: boolean
	statusConnecting: boolean
	chatSessions: ChatSessionSummary[]
	recentProjects: RecentProject[]
	currentProjectPath: string | null
	onNewChat: () => void
	onOpenProject: () => void
	onOpenSettings: () => void
	onOpenChat: (sessionId: string) => void
	onRenameChat: (sessionId: string, title: string) => void
	onDeleteChat: (sessionId: string) => void
	onOpenRecentProject: (path: string) => void
}

export function AppSidebar({
	workspaceLabel,
	sessionId,
	historyViewSessionId,
	agentWarm,
	progress,
	busy,
	statusConnecting,
	chatSessions,
	recentProjects,
	currentProjectPath,
	onNewChat,
	onOpenProject,
	onOpenSettings,
	onOpenChat,
	onRenameChat,
	onDeleteChat,
	onOpenRecentProject,
}: AppSidebarProps) {
	const otherRecents = recentProjects.filter((p) => p.path !== currentProjectPath)
	const [editingSessionId, setEditingSessionId] = useState<string | null>(null)
	const [draftTitle, setDraftTitle] = useState("")

	function startRename(chat: ChatSessionSummary) {
		setEditingSessionId(chat.sessionId)
		setDraftTitle(chat.title)
	}

	function commitRename(sessionId: string) {
		const title = draftTitle.trim()
		setEditingSessionId(null)
		if (!title) return
		onRenameChat(sessionId, title)
	}

	return (
		<>
			<div
				className="flex h-12 shrink-0 items-center justify-between border-b border-border px-4"
				data-tauri-drag-region
			>
				<span className="text-sm font-medium tracking-tight">Circulo</span>
				<button
					type="button"
					onClick={onOpenSettings}
					className="rounded p-1 text-muted transition hover:bg-white/5 hover:text-fg"
					title="Settings"
				>
					<Settings className="size-4" />
				</button>
			</div>
			<div className="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto p-3">
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

				<div className="mt-3 px-2.5 text-[11px] uppercase tracking-wider text-muted/70">
					Workspace
				</div>
				<p className="truncate px-2.5 text-xs text-muted">
					{currentProjectPath ? workspaceLabel : "No project open"}
				</p>

				{currentProjectPath ? (
					<>
						<div className="mt-4 px-2.5 text-[11px] uppercase tracking-wider text-muted/70">
							Chats
						</div>
						{chatSessions.length === 0 ? (
							<p className="px-2.5 py-1 text-xs text-muted/80">
								No saved chats yet. Start a New Chat to create one.
							</p>
						) : (
							<div className="flex flex-col gap-0.5">
								{chatSessions.map((chat) => {
									const active =
										chat.sessionId === sessionId ||
										chat.sessionId === historyViewSessionId
									const editing = editingSessionId === chat.sessionId

									return (
										<div
											key={chat.sessionId}
											className={cn(
												"group flex items-start gap-1 rounded-md transition",
												active ? "bg-white/10" : "hover:bg-white/5",
											)}
										>
											{editing ? (
												<input
													value={draftTitle}
													onChange={(event) => setDraftTitle(event.target.value)}
													onBlur={() => commitRename(chat.sessionId)}
													onKeyDown={(event) => {
														if (event.key === "Enter") {
															event.preventDefault()
															commitRename(chat.sessionId)
														}
														if (event.key === "Escape") {
															setEditingSessionId(null)
														}
													}}
													className="m-1 min-w-0 flex-1 rounded border border-border bg-black/20 px-2 py-1 text-xs text-fg outline-none focus:border-white/20"
													autoFocus
												/>
											) : (
												<button
													type="button"
													onClick={() => onOpenChat(chat.sessionId)}
													className={cn(
														"flex min-w-0 flex-1 items-start gap-2 px-2.5 py-1.5 text-left text-xs",
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
														onClick={() => startRename(chat)}
														disabled={busy}
														className="mt-1 rounded p-1 text-muted opacity-0 transition hover:bg-white/10 hover:text-fg group-hover:opacity-100 disabled:opacity-40"
														title="Rename chat"
													>
														<Pencil className="size-3.5" />
													</button>
													<button
														type="button"
														onClick={() => onDeleteChat(chat.sessionId)}
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
								})}
							</div>
						)}
					</>
				) : (
					<p className="mt-4 px-2.5 text-xs text-muted/80">
						Open a project or use General Chat to get started.
					</p>
				)}

				{otherRecents.length > 0 ? (
					<>
						<div className="mt-4 px-2.5 text-[11px] uppercase tracking-wider text-muted/70">
							Recent
						</div>
						<div className="flex flex-col gap-0.5">
							{otherRecents.map((project) => (
								<button
									key={project.path}
									type="button"
									onClick={() => onOpenRecentProject(project.path)}
									disabled={busy}
									className="truncate rounded-md px-2.5 py-1.5 text-left text-xs text-fg/80 transition hover:bg-white/5 disabled:opacity-50"
								>
									{projectLabel(project.path)}
								</button>
							))}
						</div>
					</>
				) : null}
			</div>
			<div className="shrink-0 border-t border-border px-4 py-3 text-[11px] text-muted">
				{statusConnecting
					? progress || "Opening session…"
					: agentWarm
						? "OpenCode ready · ACP"
						: progress
							? "OpenCode warming · ACP"
							: "Ready · ACP"}
			</div>
		</>
	)
}
