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
	Timer,
} from "lucide-react"
import { useAtom, useAtomValue } from "jotai"
import { useEffect, useMemo, useState, type ReactNode } from "react"
import {
	Sidebar,
	SidebarContent,
	SidebarFooter,
	SidebarGroup,
	SidebarMenu,
	SidebarMenuButton,
	SidebarMenuItem,
} from "@/components/layout/sidebar-layout"
import { SidebarFooterBar } from "@/components/layout/sidebar-footer-bar"
import { ProjectActionsMenu } from "@/components/layout/project-actions-menu"
import { SessionActionsMenu } from "@/components/layout/session-actions-menu"
import { useArchivedSessions } from "@/hooks/use-archived-sessions"
import { usePinnedSessions } from "@/hooks/use-pinned-sessions"
import { getChatsProjectPath } from "@/lib/app-settings"
import { getProjectDisplayName, isGeneralChatProject } from "@/lib/project-display"
import { useStoredProjectSessions } from "@/hooks/use-stored-project-sessions"
import {
	getProjectSidebarLabel,
	removeProjectAlias,
	setProjectAlias,
} from "@/lib/project-aliases"
import {
	addRecentProject,
	getActiveProjectPaths,
	removeRecentProject,
} from "@/lib/recent-projects"
import type { OpenProjectOptions } from "@/lib/open-project"
import { sessionTitle } from "@/lib/sessions"
import { cn } from "@/lib/utils"
import { useSessions } from "@/hooks/use-sessions"
import { isOptimisticSessionId } from "@/lib/optimistic-session"
import { appSettingsAtom, creatingSessionAtom, promptInFlightAtom } from "@/stores/atoms"
import type { SessionInfo, SidebarSessionStatus } from "@/types/acp"

interface AppSidebarProps {
	connected: boolean
	projectPath: string | null
	sessionStatus: string
	onOpenProject: (path: string, options?: OpenProjectOptions) => Promise<void>
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
	if (globalStatus === "awaiting_permission" || globalStatus === "awaiting_credential") {
		return "waiting"
	}
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
	isLoading = false,
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
	isLoading?: boolean
	status: SidebarSessionStatus
	onSelect: () => void
	pinnable?: boolean
	isPinned?: boolean
	onTogglePin?: () => void
	onArchive?: () => void
	onDelete?: () => void
	compact?: boolean
}) {
	const Icon = isLoading ? Loader2 : STATUS_ICON[status]
	const color = STATUS_COLOR[status]
	const hasPin = Boolean((pinnable || isPinned) && onTogglePin)
	const hasActions = Boolean(onArchive && onDelete)

	return (
		<SidebarMenuItem>
			<div className="group/menu-item relative">
				<SidebarMenuButton
					isActive={isSelected}
					onClick={onSelect}
					disabled={isLoading}
					size={compact ? "sm" : "default"}
					className={cn(
						hasPin && hasActions && "pr-14",
						(hasPin || hasActions) && !(hasPin && hasActions) && "pr-8",
					)}
				>
					<Icon
						className={cn(
							"size-3.5 shrink-0",
							isLoading ? "text-muted-foreground" : color,
							(status === "running" || isLoading) && "animate-spin",
						)}
					/>
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
	label,
	isExpanded,
	onToggle,
	onNewSession,
	onRename,
	onRemove,
	children,
}: {
	label: string
	isExpanded: boolean
	onToggle: () => void
	onNewSession: () => void
	onRename: (alias: string) => void
	onRemove: () => void
	children?: ReactNode
}) {
	return (
		<>
			<SidebarMenuItem>
				<div className="group/menu-item relative">
					<SidebarMenuButton onClick={onToggle} className="pr-14">
						<ChevronRight
							className="size-3 text-muted-foreground transition-transform"
							style={{ transform: isExpanded ? "rotate(90deg)" : undefined }}
						/>
						<span className="truncate font-medium">{label}</span>
					</SidebarMenuButton>
					<ProjectActionsMenu label={label} onRename={onRename} onDelete={onRemove} />
					<button
						type="button"
						title="Nueva sesión en este proyecto"
						onClick={(event) => {
							event.stopPropagation()
							onNewSession()
						}}
						className="absolute right-1 top-1/2 flex size-6 -translate-y-1/2 items-center justify-center rounded-md text-sidebar-foreground/50 opacity-0 transition-opacity hover:bg-sidebar-accent hover:text-sidebar-accent-foreground group-hover/menu-item:opacity-100"
					>
						<MessageSquarePlus className="size-3.5" />
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
	const {
		sessions,
		activeSessionId,
		newThread,
		newChat,
		newSessionInProject,
		selectSession,
		archiveSession,
		deleteSession,
	} = useSessions()
	const { pinnedIds, togglePin, isPinned } = usePinnedSessions()
	const { isArchived } = useArchivedSessions()
	const promptInFlight = useAtomValue(promptInFlightAtom)
	const creatingSession = useAtomValue(creatingSessionAtom)
	const [appSettings] = useAtom(appSettingsAtom)

	const [expandedChats, setExpandedChats] = useState(true)
	const [expandedProjects, setExpandedProjects] = useState<Record<string, boolean>>({})
	const [recentProjectsVersion, setRecentProjectsVersion] = useState(0)
	const [projectAliasesVersion, setProjectAliasesVersion] = useState(0)
	const [pendingSessionId, setPendingSessionId] = useState<string | null>(null)

	const chatsPath = getChatsProjectPath()

	const visibleSessions = useMemo(
		() => sessions.filter((session) => !isArchived(session.sessionId)),
		[sessions, isArchived],
	)

	const savedProjects = useMemo(() => {
		void recentProjectsVersion
		return getActiveProjectPaths(projectPath).filter(
			(path) => !isGeneralChatProject(path),
		)
	}, [projectPath, recentProjectsVersion])

	const storedProjectPaths = useMemo(
		() => [chatsPath, ...savedProjects],
		[chatsPath, savedProjects],
	)

	const { storedByProject, setStoredByProject } =
		useStoredProjectSessions(storedProjectPaths)

	useEffect(() => {
		if (!projectPath) return
		setStoredByProject((current) => ({
			...current,
			[projectPath]: sessions,
		}))
	}, [projectPath, sessions, setStoredByProject])

	function visibleSessionsForProject(path: string) {
		const source =
			path === projectPath && visibleSessions.length > 0
				? visibleSessions
				: (storedByProject[path] ?? [])
		return source.filter((session) => !isArchived(session.sessionId))
	}

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

	function handleDelete(sessionId: string, sessionProjectPath: string) {
		if (!window.confirm("¿Eliminar esta sesión? No se puede deshacer.")) return
		return runSessionAction(async () => {
			const result = await deleteSession(sessionProjectPath, sessionId)
			setStoredByProject((current) => ({
				...current,
				[sessionProjectPath]: result.sessions,
			}))
		})
	}

	const chatsSessions = visibleSessionsForProject(chatsPath)
	const showChatsFolder =
		appSettings.showChatsInSidebar && chatsSessions.length > 0
	const chatsLabel = getProjectDisplayName(chatsPath)

	function isProjectExpanded(path: string) {
		if (path in expandedProjects) return expandedProjects[path]
		return visibleSessionsForProject(path).length > 0 || path === projectPath
	}

	async function handleSelectSessionInProject(path: string, sessionId: string) {
		await runSessionAction(
			async () => {
				if (projectPath !== path) {
					await onOpenProject(path)
					setExpandedProjects((current) => ({ ...current, [path]: true }))
				}
				await selectSession(sessionId)
			},
			{ pendingSessionId: sessionId },
		)
	}

	function renderProjectSessions(path: string) {
		const projectSessions = visibleSessionsForProject(path)
		return (
			<div className="ml-3 border-l border-sidebar-border/10 pl-1">
				<SidebarMenu>
					{projectSessions.length === 0 ? (
						<p className="px-2 py-1.5 text-xs text-muted-foreground/60">No threads yet</p>
					) : null}
					{projectSessions.map((session, index) => (
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
							onSelect={() => void handleSelectSessionInProject(path, session.sessionId)}
							isLoading={
								pendingSessionId === session.sessionId ||
								(creatingSession && isOptimisticSessionId(session.sessionId))
							}
							pinnable
							isPinned={isPinned(session.sessionId)}
							onTogglePin={() => togglePin(session.sessionId)}
							onArchive={() => void handleArchive(session.sessionId)}
							onDelete={() => void handleDelete(session.sessionId, path)}
							compact
						/>
					))}
				</SidebarMenu>
			</div>
		)
	}

	function toggleProjectExpanded(path: string) {
		setExpandedProjects((current) => ({
			...current,
			[path]: !isProjectExpanded(path),
		}))
	}

	function getSavedProjectLabel(path: string) {
		void projectAliasesVersion
		return getProjectSidebarLabel(path)
	}

	function handleRenameProject(path: string, alias: string) {
		setProjectAlias(path, alias)
		setProjectAliasesVersion((value) => value + 1)
	}

	function handleNewSessionInProject(path: string) {
		newSessionInProject(path)
		setExpandedProjects((current) => ({ ...current, [path]: true }))
	}

	async function handleRemoveProject(path: string) {
		const label = getSavedProjectLabel(path)
		if (!window.confirm(`¿Eliminar "${label}" del sidebar?`)) return

		if (projectPath === path) {
			await runSessionAction(async () => {
				await onCloseProject()
				await onOpenProject(getChatsProjectPath())
			})
		}

		removeRecentProject(path)
		removeProjectAlias(path)
		setExpandedProjects((current) => {
			const next = { ...current }
			delete next[path]
			return next
		})
		setRecentProjectsVersion((value) => value + 1)
		setProjectAliasesVersion((value) => value + 1)
	}

	async function runSessionAction(
		action: () => Promise<void>,
		options?: { pendingSessionId?: string },
	) {
		const pendingId = options?.pendingSessionId ?? null
		if (pendingId) {
			setPendingSessionId(pendingId)
		}
		try {
			await action()
		} finally {
			if (pendingId) {
				setPendingSessionId((current) => (current === pendingId ? null : current))
			}
		}
	}

	async function handleAddProject() {
		const selected = await open({
			directory: true,
			multiple: false,
			title: "Agregar proyecto",
		})
		if (!selected || Array.isArray(selected)) return
		addRecentProject(selected)
		setRecentProjectsVersion((value) => value + 1)
		setExpandedProjects((current) => ({ ...current, [selected]: true }))
	}

	return (
		<Sidebar>
			<SidebarContent>
				<SidebarGroup>
					<SidebarMenu>
						<SidebarMenuItem>
							<SidebarMenuButton
								onClick={() => void newThread()}
								className="text-[#FAFAFA]"
							>
								<MessageSquarePlus className="size-4" />
								<span>New Thread</span>
							</SidebarMenuButton>
						</SidebarMenuItem>
						<SidebarMenuItem>
							<SidebarMenuButton
								onClick={() => newChat()}
								disabled={loading}
								className="text-[#FAFAFA]"
							>
								<MessageSquare className="size-4" />
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

				{appSettings.showPinnedInSidebar ? (
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
								onSelect={() =>
									void runSessionAction(() => selectSession(session.sessionId), {
										pendingSessionId: session.sessionId,
									})
								}
								isLoading={
								pendingSessionId === session.sessionId ||
								(creatingSession && isOptimisticSessionId(session.sessionId))
							}
								isPinned
								onTogglePin={() => togglePin(session.sessionId)}
								onArchive={() => void handleArchive(session.sessionId)}
								onDelete={() =>
									void handleDelete(session.sessionId, session.cwd || projectPath || "")
								}
							/>
							))}
						</SidebarMenu>
					</SidebarGroup>
				) : null}

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
										<span className="truncate font-medium">{chatsLabel}</span>
									</SidebarMenuButton>
								</SidebarMenuItem>
								{expandedChats ? renderProjectSessions(chatsPath) : null}
							</>
						) : null}
						{savedProjects.map((path) => {
							const isExpanded = isProjectExpanded(path)

							return (
								<ProjectFolderRow
									key={path}
									label={getSavedProjectLabel(path)}
									isExpanded={isExpanded}
									onToggle={() => toggleProjectExpanded(path)}
									onNewSession={() => void handleNewSessionInProject(path)}
									onRename={(alias) => handleRenameProject(path, alias)}
									onRemove={() => void handleRemoveProject(path)}
								>
									{isExpanded ? renderProjectSessions(path) : null}
								</ProjectFolderRow>
							)
						})}
					</SidebarMenu>
				</SidebarGroup>
			</SidebarContent>

			<SidebarFooter>
				<SidebarFooterBar connected={connected} loading={loading} />
			</SidebarFooter>
		</Sidebar>
	)
}