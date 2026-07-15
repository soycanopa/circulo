import { open } from "@tauri-apps/plugin-dialog"
import {
	AlertCircle,
	ChevronRight,
	CircleDot,
	FolderOpen,
	Folders,
	Loader2,
	MessageSquare,
	MessageSquarePlus,
	Pin,
	Settings,
	Timer,
	Trash2,
} from "lucide-react"
import { useAtomValue } from "jotai"
import { useMemo, useState, type ReactNode } from "react"
import {
	Sidebar,
	SidebarContent,
	SidebarFooter,
	SidebarGroup,
	SidebarMenu,
	SidebarMenuButton,
	SidebarMenuItem,
} from "@/components/layout/sidebar-layout"
import { ConnectionStatus } from "@/components/layout/connection-status"
import { SessionActionsMenu } from "@/components/layout/session-actions-menu"
import { useArchivedSessions } from "@/hooks/use-archived-sessions"
import { usePinnedSessions } from "@/hooks/use-pinned-sessions"
import { GENERAL_CHAT_PROJECT } from "@/lib/preferences"
import { getProjectDisplayName, isGeneralChatProject } from "@/lib/project-display"
import {
	getRecentProjectLabel,
	getRecentProjects,
	removeRecentProject,
} from "@/lib/recent-projects"
import { sessionTitle } from "@/lib/sessions"
import { cn } from "@/lib/utils"
import { useSessions } from "@/hooks/use-sessions"
import { promptInFlightAtom } from "@/stores/atoms"
import type { SessionInfo, SidebarSessionStatus } from "@/types/acp"

interface AppSidebarProps {
	connected: boolean
	projectPath: string | null
	sessionStatus: string
	onOpenProject: (path: string) => Promise<void>
	onCloseProject: () => Promise<void>
	loading: boolean
}

function sessionStatusFor(
	sessionId: string,
	activeSessionId: string | null,
	globalStatus: string,
	promptInFlight: boolean,
): SidebarSessionStatus {
	if (sessionId !== activeSessionId) return "idle"
	if (promptInFlight) return "running"
	if (globalStatus === "awaiting_permission") return "waiting"
	if (globalStatus === "disconnected") return "failed"
	return "idle"
}

const STATUS_ICON = {
	running: Loader2,
	waiting: Timer,
	idle: CircleDot,
	failed: AlertCircle,
} as const

const STATUS_COLOR = {
	running: "text-green-500",
	waiting: "text-yellow-500",
	idle: "text-muted-foreground",
	failed: "text-red-500",
} as const

function SessionItem({
	session,
	sessionIndex,
	isSelected,
	status,
	onSelect,
	pinnable = false,
	isPinned = false,
	onTogglePin,
	onArchive,
	onDelete,
	compact = false,
}: {
	session: SessionInfo
	sessionIndex: number
	isSelected: boolean
	status: SidebarSessionStatus
	onSelect: () => void
	pinnable?: boolean
	isPinned?: boolean
	onTogglePin?: () => void
	onArchive?: () => void
	onDelete?: () => void
	compact?: boolean
}) {
	const Icon = STATUS_ICON[status]
	const color = STATUS_COLOR[status]
	const hasPin = Boolean((pinnable || isPinned) && onTogglePin)
	const hasActions = Boolean(onArchive && onDelete)

	return (
		<SidebarMenuItem>
			<div className="group/menu-item relative">
				<SidebarMenuButton
					isActive={isSelected}
					onClick={onSelect}
					size={compact ? "sm" : "default"}
					className={cn(
						hasPin && hasActions && "pr-14",
						(hasPin || hasActions) && !(hasPin && hasActions) && "pr-8",
					)}
				>
					<Icon className={cn("size-3.5 shrink-0", color, status === "running" && "animate-spin")} />
					<span className="min-w-0 flex-1 truncate">{sessionTitle(session, sessionIndex)}</span>
				</SidebarMenuButton>
				{hasActions ? (
					<SessionActionsMenu
						className={hasPin ? "right-8" : undefined}
						onArchive={onArchive!}
						onDelete={onDelete!}
					/>
				) : null}
				{hasPin ? (
					<button
						type="button"
						title={isPinned ? "Quitar de pinned" : "Agregar a pinned"}
						onClick={(event) => {
							event.stopPropagation()
							onTogglePin!()
						}}
						className={cn(
							"absolute right-1 top-1/2 flex size-6 -translate-y-1/2 items-center justify-center rounded-md text-sidebar-foreground/50 opacity-0 transition-opacity hover:bg-sidebar-accent hover:text-sidebar-accent-foreground group-hover/menu-item:opacity-100",
							isPinned && "text-sidebar-ring",
						)}
					>
						<Pin className={cn("size-3", isPinned && "fill-current")} />
					</button>
				) : null}
			</div>
		</SidebarMenuItem>
	)
}

function ProjectFolderRow({
	path,
	isActive,
	isExpanded,
	onToggle,
	onRemove,
	children,
}: {
	path: string
	isActive: boolean
	isExpanded: boolean
	onToggle: () => void
	onRemove: () => void
	children?: ReactNode
}) {
	const label = getRecentProjectLabel(path)

	return (
		<>
			<SidebarMenuItem>
				<div className="group/menu-item relative">
					<SidebarMenuButton isActive={isActive} onClick={onToggle} className="pr-8">
						<ChevronRight
							className="size-3 text-muted-foreground transition-transform"
							style={{ transform: isExpanded ? "rotate(90deg)" : undefined }}
						/>
						<span className="truncate font-medium">{label}</span>
					</SidebarMenuButton>
					<button
						type="button"
						title="Eliminar proyecto"
						onClick={(event) => {
							event.stopPropagation()
							onRemove()
						}}
						className="absolute right-1 top-1/2 flex size-6 -translate-y-1/2 items-center justify-center rounded-md text-sidebar-foreground/50 opacity-0 transition-opacity hover:bg-sidebar-accent hover:text-destructive group-hover/menu-item:opacity-100"
					>
						<Trash2 className="size-3.5" />
					</button>
				</div>
			</SidebarMenuItem>
			{isExpanded ? children : null}
		</>
	)
}

export function AppSidebar({
	connected,
	projectPath,
	sessionStatus,
	onOpenProject,
	onCloseProject,
	loading,
}: AppSidebarProps) {
	const { sessions, activeSessionId, newThread, newChat, selectSession, archiveSession, deleteSession } =
		useSessions()
	const { pinnedIds, togglePin, isPinned } = usePinnedSessions()
	const { isArchived } = useArchivedSessions()
	const promptInFlight = useAtomValue(promptInFlightAtom)
	const [expandedChats, setExpandedChats] = useState(true)
	const [expandedProjects, setExpandedProjects] = useState<Record<string, boolean>>({})
	const [recentProjectsVersion, setRecentProjectsVersion] = useState(0)
	const [sessionPending, setSessionPending] = useState(false)

	const visibleSessions = useMemo(
		() => sessions.filter((session) => !isArchived(session.sessionId)),
		[sessions, isArchived],
	)

	const pinnedSessions = useMemo(
		() =>
			pinnedIds
				.map((id) => visibleSessions.find((session) => session.sessionId === id))
				.filter((session): session is SessionInfo => Boolean(session)),
		[pinnedIds, visibleSessions],
	)

	function handleArchive(sessionId: string) {
		return runSessionAction(() => archiveSession(sessionId))
	}

	function handleDelete(sessionId: string) {
		if (!window.confirm("¿Eliminar esta sesión? No se puede deshacer.")) return
		return runSessionAction(() => deleteSession(sessionId))
	}

	const projectName = getProjectDisplayName(projectPath)
	const isGeneralChat = isGeneralChatProject(projectPath)
	const showChatsFolder = isGeneralChat && visibleSessions.length > 0
	const savedProjects = useMemo(() => {
		void recentProjectsVersion
		const recent = getRecentProjects()
		if (projectPath && !isGeneralChatProject(projectPath) && !recent.includes(projectPath)) {
			return [projectPath, ...recent]
		}
		return recent
	}, [projectPath, recentProjectsVersion])

	function isProjectExpanded(path: string) {
		if (path in expandedProjects) return expandedProjects[path]
		return path === projectPath
	}

	function toggleProjectExpanded(path: string) {
		setExpandedProjects((current) => ({
			...current,
			[path]: !isProjectExpanded(path),
		}))
	}

	async function handleRemoveProject(path: string) {
		const label = getRecentProjectLabel(path)
		if (!window.confirm(`¿Eliminar "${label}" del sidebar?`)) return
		removeRecentProject(path)
		setRecentProjectsVersion((value) => value + 1)
		if (projectPath !== path) return
		await runSessionAction(async () => {
			await onCloseProject()
			await onOpenProject(GENERAL_CHAT_PROJECT)
		})
	}

	async function runSessionAction(action: () => Promise<void>) {
		setSessionPending(true)
		try {
			await action()
		} finally {
			setSessionPending(false)
		}
	}

	async function handleAddProject() {
		const selected = await open({ directory: true, multiple: false, title: "Abrir proyecto" })
		if (!selected || Array.isArray(selected)) return
		await onOpenProject(selected)
		setRecentProjectsVersion((value) => value + 1)
	}

	return (
		<Sidebar>
			<SidebarContent>
				<SidebarGroup>
					<SidebarMenu>
						<SidebarMenuItem>
							<SidebarMenuButton
								onClick={() => void runSessionAction(newThread)}
								disabled={sessionPending}
								className="text-[#FAFAFA]"
							>
								{sessionPending ? (
									<Loader2 className="size-4 animate-spin" />
								) : (
									<MessageSquarePlus className="size-4" />
								)}
								<span>New Thread</span>
							</SidebarMenuButton>
						</SidebarMenuItem>
						<SidebarMenuItem>
							<SidebarMenuButton
								onClick={() => void runSessionAction(newChat)}
								disabled={sessionPending}
								className="text-[#FAFAFA]"
							>
								{sessionPending ? (
									<Loader2 className="size-4 animate-spin" />
								) : (
									<MessageSquare className="size-4" />
								)}
								<span>New Chat</span>
							</SidebarMenuButton>
						</SidebarMenuItem>
						<SidebarMenuItem>
							<SidebarMenuButton onClick={() => void handleAddProject()} className="text-[#FAFAFA]">
								<FolderOpen className="size-4" />
								<span>Add Project</span>
							</SidebarMenuButton>
						</SidebarMenuItem>
					</SidebarMenu>
				</SidebarGroup>

				<SidebarGroup label="Pinned" icon={Pin}>
					<SidebarMenu>
						{pinnedSessions.map((session) => (
							<SessionItem
								key={session.sessionId}
								session={session}
								sessionIndex={visibleSessions.findIndex((s) => s.sessionId === session.sessionId)}
								isSelected={session.sessionId === activeSessionId}
								status={sessionStatusFor(
									session.sessionId,
									activeSessionId,
									sessionStatus,
									promptInFlight,
								)}
								onSelect={() => void runSessionAction(() => selectSession(session.sessionId))}
								isPinned
								onTogglePin={() => togglePin(session.sessionId)}
								onArchive={() => void handleArchive(session.sessionId)}
								onDelete={() => void handleDelete(session.sessionId)}
							/>
						))}
					</SidebarMenu>
				</SidebarGroup>

				<SidebarGroup label="Projects" icon={Folders}>
					<SidebarMenu>
						{showChatsFolder ? (
							<>
								<SidebarMenuItem>
									<SidebarMenuButton onClick={() => setExpandedChats((value) => !value)}>
										<ChevronRight
											className="size-3 text-muted-foreground transition-transform"
											style={{ transform: expandedChats ? "rotate(90deg)" : undefined }}
										/>
										<span className="truncate font-medium">{projectName}</span>
									</SidebarMenuButton>
								</SidebarMenuItem>
								{expandedChats ? (
									<div className="ml-3 border-l border-sidebar-border/10 pl-1">
										<SidebarMenu>
											{visibleSessions.map((session, index) => (
												<SessionItem
													key={session.sessionId}
													session={session}
													sessionIndex={index}
													isSelected={session.sessionId === activeSessionId}
													status={sessionStatusFor(
														session.sessionId,
														activeSessionId,
														sessionStatus,
														promptInFlight,
													)}
													onSelect={() =>
														void runSessionAction(() => selectSession(session.sessionId))
													}
													pinnable
													isPinned={isPinned(session.sessionId)}
													onTogglePin={() => togglePin(session.sessionId)}
													onArchive={() => void handleArchive(session.sessionId)}
													onDelete={() => void handleDelete(session.sessionId)}
													compact
												/>
											))}
										</SidebarMenu>
									</div>
								) : null}
							</>
						) : null}
						{savedProjects.map((path) => {
							const isActive = path === projectPath
							const isExpanded = isProjectExpanded(path)

							return (
								<ProjectFolderRow
									key={path}
									path={path}
									isActive={isActive}
									isExpanded={isExpanded}
									onToggle={() => {
										if (!isActive) {
											void runSessionAction(async () => {
												await onOpenProject(path)
												setExpandedProjects((current) => ({ ...current, [path]: true }))
											})
											return
										}
										toggleProjectExpanded(path)
									}}
									onRemove={() => void handleRemoveProject(path)}
								>
									<div className="ml-3 border-l border-sidebar-border/10 pl-1">
										<SidebarMenu>
											{isActive && visibleSessions.length === 0 ? (
												<p className="px-2 py-1.5 text-xs text-muted-foreground/60">
													No threads yet
												</p>
											) : null}
											{isActive
												? visibleSessions.map((session, index) => (
														<SessionItem
															key={session.sessionId}
															session={session}
															sessionIndex={index}
															isSelected={session.sessionId === activeSessionId}
															status={sessionStatusFor(
																session.sessionId,
																activeSessionId,
																sessionStatus,
																promptInFlight,
															)}
															onSelect={() =>
																void runSessionAction(() => selectSession(session.sessionId))
															}
															pinnable
															isPinned={isPinned(session.sessionId)}
															onTogglePin={() => togglePin(session.sessionId)}
															onArchive={() => void handleArchive(session.sessionId)}
															onDelete={() => void handleDelete(session.sessionId)}
															compact
														/>
													))
												: null}
										</SidebarMenu>
									</div>
								</ProjectFolderRow>
							)
						})}
					</SidebarMenu>
				</SidebarGroup>
			</SidebarContent>

			<SidebarFooter>
				<div className="flex items-center justify-between gap-1">
					<ConnectionStatus connected={connected} />
					<SidebarMenu>
						<SidebarMenuItem>
							<SidebarMenuButton className="text-muted-foreground" disabled={loading}>
								<Settings className="size-4" />
								<span>Settings</span>
							</SidebarMenuButton>
						</SidebarMenuItem>
					</SidebarMenu>
				</div>
			</SidebarFooter>
		</Sidebar>
	)
}