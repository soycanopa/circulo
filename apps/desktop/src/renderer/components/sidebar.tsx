import { Collapsible, CollapsibleContent } from "@circulo/ui/components/collapsible"
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuSeparator,
	ContextMenuTrigger,
} from "@circulo/ui/components/context-menu"
import { Input } from "@circulo/ui/components/input"
import {
	SidebarContent,
	SidebarFooter,
	SidebarGroup,
	SidebarGroupContent,
	SidebarGroupLabel,
	SidebarMenu,
	SidebarMenuButton,
	SidebarMenuItem,
	SidebarSeparator,
} from "@circulo/ui/components/sidebar"
import { Tooltip, TooltipContent, TooltipTrigger } from "@circulo/ui/components/tooltip"
import { useNavigate, useParams } from "@tanstack/react-router"
import { useAtomValue, useSetAtom } from "jotai"
import {
	ArchiveIcon,
	BotIcon,
	ChevronDownIcon,
	CommandIcon,
	FolderGit2,
	FolderIcon,
	GitForkIcon,
	Loader2Icon,
	MessageCircleIcon,
	MessageCirclePlusIcon,
	PencilIcon,
	SearchIcon,
	SettingsIcon,
	TrashIcon,
	XIcon,
} from "lucide-react"
import { memo, useCallback, useEffect, useMemo, useRef, useState, useTransition } from "react"
import { activeServerConfigAtom } from "../atoms/connection"
import { agentFamily, projectSessionIdsFamily, sandboxMappingsAtom } from "../atoms/derived/agents"
import { automationsEnabledAtom } from "../atoms/feature-flags"
import { projectPaginationFamily, markSessionViewedAtom } from "../atoms/sessions"
import { appStore } from "../atoms/store"
import type { Agent, AgentStatus, SidebarProject } from "../lib/types"
import { loadMoreProjectSessions, loadProjectSessions } from "../services/connection-manager"
import { ServerIndicator } from "./server-indicator"

// ============================================================
// Constants
// ============================================================

const STATUS_COLOR: Record<AgentStatus, string> = {
	running: "text-green-500",
	waiting: "text-yellow-500",
	paused: "text-muted-foreground",
	completed: "text-muted-foreground",
	failed: "text-red-500",
	idle: "text-muted-foreground",
}

const ARCHIVED_PROJECTS_KEY = "circulo-archived-projects"

function getArchivedProjectIds(): Set<string> {
	try {
		const raw = localStorage.getItem(ARCHIVED_PROJECTS_KEY)
		if (!raw) return new Set()
		return new Set(JSON.parse(raw) as string[])
	} catch {
		return new Set()
	}
}

function archiveProject(id: string) {
	const archived = getArchivedProjectIds()
	archived.add(id)
	localStorage.setItem(ARCHIVED_PROJECTS_KEY, JSON.stringify([...archived]))
}

/** Detect if a project ID looks like a git commit hash (pure hex, 40 chars) */
function isGitProjectId(id: string): boolean {
	return /^[a-f0-9]{40}$/.test(id)
}

// ============================================================
// Props
// ============================================================

interface AppSidebarContentProps {
	agents: Agent[]
	projects: SidebarProject[]
	onOpenCommandPalette: () => void
	onRenameSession?: (agent: Agent, title: string) => Promise<void>
	onDeleteSession?: (agent: Agent) => Promise<void>
	onForkSession?: (agent: Agent) => Promise<void>
	serverConnected: boolean
	homeDirectory?: string | null
	onNavigateChat?: () => void
}

// ============================================================
// Main component
// ============================================================

/**
 * Default sidebar content: Active Now, Threads (projects + sessions), Chat + Settings footer.
 * Rendered inside the `<Sidebar>` shell provided by `SidebarLayout`.
 */
export function AppSidebarContent({
	agents,
	projects,
	onOpenCommandPalette,
	onRenameSession,
	onDeleteSession,
	onForkSession,
	serverConnected,
	homeDirectory,
	onNavigateChat,
}: AppSidebarContentProps) {
	const navigate = useNavigate()
	const routeParams = useParams({ strict: false }) as { sessionId?: string }
	const selectedSessionId = routeParams.sessionId ?? null
	const automationsEnabled = useAtomValue(automationsEnabledAtom)
	const activeServer = useAtomValue(activeServerConfigAtom)
	const isLocalServer = activeServer.type === "local"

	// --- Project search state ---
	const [projectSearch, setProjectSearch] = useState("")
	const [projectSearchActive, setProjectSearchActive] = useState(false)
	const projectSearchRef = useRef<HTMLInputElement>(null)

	// Filter out archived projects
	const archivedIds = useMemo(() => getArchivedProjectIds(), [])
	const visibleProjects = useMemo(
		() => projects.filter((p) => !archivedIds.has(p.id)),
		[projects, archivedIds],
	)

	// Filter projects by search query (client-side, case-insensitive)
	const filteredProjects = useMemo(() => {
		if (!projectSearch.trim()) return visibleProjects
		const q = projectSearch.toLowerCase()
		return visibleProjects.filter(
			(p) => p.name.toLowerCase().includes(q) || p.directory.toLowerCase().includes(q),
		)
	}, [visibleProjects, projectSearch])

	const toggleProjectSearch = useCallback(() => {
		setProjectSearchActive((prev) => {
			if (prev) {
				setProjectSearch("")
				return false
			}
			return true
		})
	}, [])

	// Auto-focus search input when activated
	useEffect(() => {
		if (projectSearchActive && projectSearchRef.current) {
			projectSearchRef.current.focus()
		}
	}, [projectSearchActive])

	// Chat sessions: agents whose directory equals homeDirectory
	const chatSessionIds = useMemo(() => {
		if (!homeDirectory) return new Set<string>()
		return new Set(
			agents.filter((a) => !a.parentId && a.directory === homeDirectory).map((a) => a.id),
		)
	}, [agents, homeDirectory])

	// Active sessions: non-parent, non-chat, running/waiting/failed
	const activeSessions = useMemo(
		() =>
			agents
				.filter(
					(a) =>
						!a.parentId &&
						!chatSessionIds.has(a.id) &&
						(a.status === "running" || a.status === "waiting" || a.status === "failed"),
				)
				.sort((a, b) => b.createdAt - a.createdAt),
		[agents, chatSessionIds],
	)

	const hasThreads = visibleProjects.length > 0 || chatSessionIds.size > 0
	const hasContent = agents.length > 0 || hasThreads
	const showEmptyState = !hasContent

	const handleArchive = useCallback(
		(e: React.MouseEvent, project: SidebarProject) => {
			e.stopPropagation()
			archiveProject(project.id)
		},
		[],
	)

	return (
		<>
			{/* Scrollable content */}
			<SidebarContent>
				{/* Empty state */}
				{showEmptyState && (
					<div className="flex flex-1 items-center justify-center p-4">
						<div className="space-y-2 text-center">
							{!serverConnected ? (
								<>
									<p className="text-sm text-muted-foreground">Server offline</p>
									<p className="text-xs text-muted-foreground/60">
										Check your connection in Settings
									</p>
								</>
							) : (
								<>
									<p className="text-sm text-muted-foreground">No threads yet</p>
									<p className="text-xs text-muted-foreground/60">Add a project to get started</p>
								</>
							)}
						</div>
					</div>
				)}

			{/* New Thread + Chat + Automations */}
			<SidebarGroup>
				<SidebarGroupContent>
					<SidebarMenu>
						<SidebarMenuItem>
							<SidebarMenuButton
								tooltip="New Thread"
								onClick={() => navigate({ to: "/" })}
								className="text-muted-foreground"
							>
								<MessageCirclePlusIcon className="size-4" />
								<span>New Thread</span>
							</SidebarMenuButton>
						</SidebarMenuItem>
						<SidebarMenuItem>
							<SidebarMenuButton
								tooltip="Chat"
								onClick={onNavigateChat}
								className="text-muted-foreground"
							>
								<MessageCircleIcon className="size-4" />
							<span>New Chat</span>
						</SidebarMenuButton>
					</SidebarMenuItem>
					{automationsEnabled && isLocalServer && (
							<SidebarMenuItem>
								<SidebarMenuButton
									tooltip="Automations"
									onClick={() => navigate({ to: "/automations" })}
									className="text-muted-foreground"
								>
									<BotIcon className="size-4" />
									<span>Automations</span>
								</SidebarMenuButton>
							</SidebarMenuItem>
						)}
					</SidebarMenu>
				</SidebarGroupContent>
			</SidebarGroup>

				{/* Active Now */}
				{activeSessions.length > 0 && (
					<SidebarGroup>
						<SidebarGroupLabel>Active Now</SidebarGroupLabel>
						<SidebarGroupContent>
							<SidebarMenu>
								{activeSessions.map((agent) => (
									<SessionItem
										key={agent.id}
										agent={agent}
										isSelected={agent.id === selectedSessionId}
										onRename={onRenameSession}
										onDelete={onDeleteSession}
										onFork={onForkSession}
										showProject
									/>
								))}
							</SidebarMenu>
						</SidebarGroupContent>
					</SidebarGroup>
				)}

				{/* Threads */}
				{activeSessions.length > 0 && hasThreads && (
					<SidebarSeparator className="bg-sidebar-border/5" />
				)}
				{hasThreads && (
					<SidebarGroup>
						<SidebarGroupLabel>Threads</SidebarGroupLabel>
						{/* Action buttons row */}
						<div className="absolute top-3.5 right-3 flex max-w-[calc(100%-4rem)] items-center gap-0.5 overflow-hidden">
							<Tooltip>
								<TooltipTrigger
									render={
										<button
											type="button"
											onClick={toggleProjectSearch}
											className={`text-sidebar-foreground ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground flex aspect-square w-5 items-center justify-center rounded-md p-0 transition-colors ${
												projectSearchActive
													? "bg-sidebar-accent text-sidebar-accent-foreground"
													: ""
											}`}
										/>
									}
								>
									{projectSearchActive ? (
										<XIcon className="size-4 shrink-0" />
									) : (
										<SearchIcon className="size-4 shrink-0" />
									)}
									<span className="sr-only">Search threads</span>
								</TooltipTrigger>
								<TooltipContent side="bottom">
									{projectSearchActive ? "Close search" : "Search threads"}
								</TooltipContent>
							</Tooltip>
							<Tooltip>
								<TooltipTrigger
									render={
										<button
											type="button"
											onClick={onOpenCommandPalette}
											className="text-sidebar-foreground ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground flex aspect-square w-5 shrink-0 items-center justify-center rounded-md p-0 transition-colors"
										/>
									}
								>
									<CommandIcon className="size-4 shrink-0" />
									<span className="sr-only">Command palette</span>
								</TooltipTrigger>
								<TooltipContent side="bottom">Command palette (&#8984;K)</TooltipContent>
							</Tooltip>
						</div>

						{/* Inline project search */}
						{projectSearchActive && (
							<div className="px-2 pb-1">
								<Input
									ref={projectSearchRef}
									value={projectSearch}
									onChange={(e) => setProjectSearch(e.target.value)}
									onKeyDown={(e) => {
										if (e.key === "Escape") {
											toggleProjectSearch()
										}
									}}
									placeholder="Filter threads..."
									className="h-7 text-xs"
								/>
							</div>
						)}

						<SidebarGroupContent>
							<SidebarMenu>
								{/* Chats group */}
								{chatSessionIds.size > 0 && homeDirectory && (
									<ProjectFolder
										project={{
											id: "chats",
											slug: "chats",
											name: "Chats",
											directory: homeDirectory,
											agentCount: chatSessionIds.size,
											lastActiveAt: 0,
											hasActiveAgent: false,
										}}
										selectedSessionId={selectedSessionId}
										onRename={onRenameSession}
										onDelete={onDeleteSession}
										onFork={onForkSession}
										isChatsFolder
									/>
								)}
								{filteredProjects.map((project) => (
									<ProjectFolder
										key={project.id}
										project={project}
										selectedSessionId={selectedSessionId}
										onRename={onRenameSession}
										onDelete={onDeleteSession}
										onFork={onForkSession}
										onArchive={handleArchive}
									/>
								))}
								{projectSearch && filteredProjects.length === 0 && (
									<p className="px-2 py-1.5 text-xs text-muted-foreground/60">
										No threads match &ldquo;{projectSearch}&rdquo;
									</p>
								)}
							</SidebarMenu>
						</SidebarGroupContent>
					</SidebarGroup>
				)}

			</SidebarContent>
			<SidebarFooter className="flex flex-row items-center justify-between p-2">
				<SidebarMenuButton
					tooltip="Settings"
					onClick={() => navigate({ to: "/settings" })}
					className="text-muted-foreground shrink-0"
				>
					<SettingsIcon className="size-4" />
					<span>Settings</span>
				</SidebarMenuButton>
				<ServerIndicator />
			</SidebarFooter>
		</>
	)
}

// ============================================================
// Sub-components
// ============================================================

/**
 * Wrapper that subscribes to a single agent via agentFamily and renders
 * a SessionItem. Used by ProjectFolder so each item only re-renders
 * when its own agent changes, not when any agent in the project changes.
 */
const ProjectSessionItem = memo(function ProjectSessionItem({
	sessionId,
	selectedSessionId,
	onRename,
	onDelete,
	onFork,
}: {
	sessionId: string
	selectedSessionId: string | null
	onRename?: (agent: Agent, title: string) => Promise<void>
	onDelete?: (agent: Agent) => Promise<void>
	onFork?: (agent: Agent) => Promise<void>
}) {
	const agent = useAtomValue(agentFamily(sessionId))
	if (!agent) return null
	return (
		<SessionItem
			agent={agent}
			isSelected={agent.id === selectedSessionId}
			onRename={onRename}
			onDelete={onDelete}
			onFork={onFork}
			compact
		/>
	)
})

/**
 * A project folder in the sidebar that lists its sessions as a flat list.
 * Sessions are loaded lazily on first expand from the server.
 * Shows a "Load more" button that fetches additional sessions.
 */
const ProjectFolder = memo(function ProjectFolder({
	project,
	selectedSessionId,
	onRename,
	onDelete,
	onFork,
	onArchive,
	isChatsFolder = false,
}: {
	project: SidebarProject
	selectedSessionId: string | null
	onRename?: (agent: Agent, title: string) => Promise<void>
	onDelete?: (agent: Agent) => Promise<void>
	onFork?: (agent: Agent) => Promise<void>
	onArchive?: (e: React.MouseEvent, project: SidebarProject) => void
	isChatsFolder?: boolean
}) {
	const navigate = useNavigate()
	const [expanded, setExpanded] = useState(false)

	// Subscribe to just this project's session IDs
	const sessionIds = useAtomValue(projectSessionIdsFamily(project.directory))

	// Per-project pagination state from the server
	const pagination = useAtomValue(projectPaginationFamily(project.directory))

	// Load sessions on first expand
	useEffect(() => {
		if (!expanded || pagination.loaded || pagination.loading) return

		// Look up sandbox dirs for this project from the discovery data
		const { parentToSandboxes } = appStore.get(sandboxMappingsAtom)
		const sandboxDirs = parentToSandboxes.get(project.directory)

		loadProjectSessions(project.directory, sandboxDirs?.size ? sandboxDirs : undefined, {
			limit: 5,
			roots: true,
		})
	}, [expanded, pagination.loaded, pagination.loading, project.directory])

	// Read agents non-reactively (via appStore.get) for sorting.
	// Individual items render reactively via ProjectSessionItem -> agentFamily.
	const projectSessions = useMemo(() => {
		const agents: Agent[] = []
		for (const id of sessionIds) {
			const agent = appStore.get(agentFamily(id))
			if (agent) agents.push(agent)
		}
		return agents.sort((a, b) => {
			// Active sessions float to top
			const aActive = a.status === "running" || a.status === "waiting" || a.status === "failed"
			const bActive = b.status === "running" || b.status === "waiting" || b.status === "failed"
			if (aActive !== bActive) return aActive ? -1 : 1
			return b.lastActiveAt - a.lastActiveAt
		})
	}, [sessionIds])

	const handleLoadMore = useCallback(() => {
		loadMoreProjectSessions(project.directory, pagination.currentLimit)
	}, [project.directory, pagination.currentLimit])

	// Show loading state when initial fetch or load-more is in progress
	const isInitialLoading = expanded && !pagination.loaded && !pagination.loading
	const isLoading = pagination.loading || isInitialLoading

	const ProjectIcon = isChatsFolder ? MessageCircleIcon : isGitProjectId(project.id) ? FolderGit2 : FolderIcon

	return (
		<SidebarMenuItem>
			<Collapsible open={expanded} onOpenChange={setExpanded}>
				<div className="group/project flex items-center">
					<SidebarMenuButton
						tooltip={project.name}
						onClick={() => {
							setExpanded(!expanded)
							if (!isChatsFolder) {
								navigate({
									to: "/project/$projectSlug",
									params: { projectSlug: project.slug },
								})
							}
						}}
						className="flex-1 min-w-0"
					>
						<ProjectIcon className="size-3 shrink-0 text-muted-foreground" />
						<span className="truncate font-medium">{project.name}</span>
					</SidebarMenuButton>
					{onArchive && !isChatsFolder && (
						<button
							type="button"
							onClick={(e) => onArchive(e, project)}
							className="opacity-0 group-hover/project:opacity-100 shrink-0 mr-1 text-sidebar-foreground ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground flex aspect-square w-5 items-center justify-center rounded-md p-0 transition-all"
						>
							<ArchiveIcon className="size-3.5" />
							<span className="sr-only">Archive {project.name}</span>
						</button>
					)}
				</div>

				<CollapsibleContent
					keepMounted
					className="flex h-[var(--collapsible-panel-height)] flex-col overflow-hidden transition-[height] duration-200 ease-out data-[ending-style]:h-0 data-[starting-style]:h-0 [&[hidden]:not([hidden='until-found'])]:hidden"
				>
					<div className="ml-3 border-l border-sidebar-border/5 pl-1">
						{isLoading && projectSessions.length === 0 ? (
							<p className="flex items-center gap-1.5 px-2 py-1.5 text-xs text-muted-foreground/60">
								<Loader2Icon className="size-3 animate-spin" />
								Loading sessions...
							</p>
						) : pagination.loaded && projectSessions.length === 0 ? (
							<p className="px-2 py-1.5 text-xs text-muted-foreground/60">No sessions yet</p>
						) : (
							<SidebarMenu>
								{projectSessions.map((agent) => (
									<ProjectSessionItem
										key={agent.id}
										sessionId={agent.id}
										selectedSessionId={selectedSessionId}
										onRename={onRename}
										onDelete={onDelete}
										onFork={onFork}
									/>
								))}
								{pagination.loaded && pagination.hasMore && (
									<button
										type="button"
										onClick={handleLoadMore}
										disabled={pagination.loading}
										className="w-full cursor-pointer px-2 py-1 text-left text-[11px] text-muted-foreground transition-colors hover:text-foreground disabled:cursor-default disabled:opacity-50"
									>
										{pagination.loading ? (
											<span className="flex items-center gap-1">
												<Loader2Icon className="size-3 animate-spin" />
												Loading...
											</span>
										) : (
											<span className="flex items-center gap-1">
												<ChevronDownIcon className="size-3" />
												Load more sessions
											</span>
										)}
									</button>
								)}
							</SidebarMenu>
						)}
					</div>
				</CollapsibleContent>
			</Collapsible>
		</SidebarMenuItem>
	)
})

// ============================================================
// Session item
// ============================================================

/**
 * Hook that returns a live-updating relative "last active" time string.
 * For active (running/waiting) sessions, ticks every minute.
 * For idle/completed sessions, returns the static duration from the agent atom.
 */
function useLiveLastActive(agent: Agent): string {
	const isActive = agent.status === "running" || agent.status === "waiting"

	const [display, setDisplay] = useState(agent.duration)

	useEffect(() => {
		if (!isActive) {
			setDisplay(agent.duration)
			return
		}

		// Active sessions: show "now" and tick every 60s to stay fresh
		setDisplay("now")
		const id = setInterval(() => setDisplay("now"), 60_000)
		return () => clearInterval(id)
	}, [isActive, agent.duration])

	return display
}

const SessionItem = memo(function SessionItem({
	agent,
	isSelected,
	onRename,
	onDelete,
	onFork,
	showProject = false,
	compact = false,
}: {
	agent: Agent
	isSelected: boolean
	onRename?: (agent: Agent, title: string) => Promise<void>
	onDelete?: (agent: Agent) => Promise<void>
	onFork?: (agent: Agent) => Promise<void>
	showProject?: boolean
	compact?: boolean
}) {
	const navigate = useNavigate()
	const [, startTransition] = useTransition()
	const markViewed = useSetAtom(markSessionViewedAtom)
	const statusColor = STATUS_COLOR[agent.status]
	const isWorktree = !!agent.worktreePath
	const lastActive = useLiveLastActive(agent)
	const hasNewActivity =
		agent.lastActiveAt > (agent.lastViewedAt ?? 0) && agent.status !== "running"

	const [isEditing, setIsEditing] = useState(false)
	const [editValue, setEditValue] = useState(agent.name)
	const inputRef = useRef<HTMLInputElement>(null)

	const onSelect = useCallback(() => {
		markViewed(agent.id)
		startTransition(() => {
			navigate({
				to: "/project/$projectSlug/session/$sessionId",
				params: { projectSlug: agent.projectSlug, sessionId: agent.id },
			})
		})
	}, [navigate, agent.projectSlug, agent.id, markViewed])

	const startEditing = useCallback(() => {
		setEditValue(agent.name)
		setIsEditing(true)
	}, [agent.name])

	const confirmRename = useCallback(async () => {
		const trimmed = editValue.trim()
		setIsEditing(false)
		if (trimmed && trimmed !== agent.name && onRename) {
			await onRename(agent, trimmed)
		}
	}, [editValue, agent, onRename])

	const cancelEditing = useCallback(() => {
		setIsEditing(false)
		setEditValue(agent.name)
	}, [agent.name])

	useEffect(() => {
		if (isEditing && inputRef.current) {
			inputRef.current.focus()
			inputRef.current.select()
		}
	}, [isEditing])

	const tooltipLabel = showProject ? agent.project : agent.name

	const btn = (
		<SidebarMenuItem>
			<SidebarMenuButton
				isActive={isSelected}
				tooltip={tooltipLabel}
				size={compact ? "sm" : "default"}
				onClick={isEditing ? undefined : onSelect}
			>
				{isWorktree ? (
					<span className="relative shrink-0">
						<GitForkIcon
							className={`${statusColor} ${agent.status === "running" ? "animate-pulse" : ""}`}
						/>
						{hasNewActivity && (
							<span
								aria-hidden="true"
								className="absolute -right-0.5 -top-0.5 size-1.5 rounded-full bg-blue-500 ring-1 ring-background"
							/>
						)}
					</span>
				) : (
					<span className="relative shrink-0">
						<MessageCircleIcon
							className={`${statusColor} ${agent.status === "running" ? "animate-spin" : ""}`}
						/>
						{hasNewActivity && (
							<span
								aria-hidden="true"
								className="absolute -right-0.5 -top-0.5 size-1.5 rounded-full bg-blue-500 ring-1 ring-background"
							/>
						)}
					</span>
				)}

				{isEditing ? (
					<Input
						ref={inputRef}
						value={editValue}
						onChange={(e) => setEditValue(e.target.value)}
						onKeyDown={(e) => {
							e.stopPropagation()
							if (e.key === "Enter") confirmRename()
							if (e.key === "Escape") cancelEditing()
						}}
						onBlur={confirmRename}
						onClick={(e) => e.stopPropagation()}
						className={`h-auto min-w-0 flex-1 border-none bg-transparent p-0 shadow-none focus-visible:ring-0 ${compact ? "text-xs" : "text-[13px]"}`}
					/>
				) : (
					<div className="min-w-0 flex-1">
						<span className={`block truncate leading-tight ${compact ? "text-xs" : "text-[13px]"}`}>
							{agent.name}
						</span>

						{agent.status === "waiting" && agent.currentActivity && (
							<span className="block truncate text-[11px] leading-tight text-yellow-500">
								{agent.currentActivity}
							</span>
						)}
					</div>
				)}

				{!isEditing && (
					<span className="shrink-0 text-xs tabular-nums text-muted-foreground">{lastActive}</span>
				)}
			</SidebarMenuButton>
		</SidebarMenuItem>
	)

	return (
		<ContextMenu>
			<ContextMenuTrigger render={btn} />
			<ContextMenuContent>
				{onRename && (
					<ContextMenuItem onSelect={startEditing}>
						<PencilIcon className="size-4" />
						Rename
					</ContextMenuItem>
				)}
				{onFork && (
					<ContextMenuItem onSelect={() => onFork(agent)}>
						<GitForkIcon className="size-4" />
						Fork
					</ContextMenuItem>
				)}
				{(onRename || onFork) && onDelete && <ContextMenuSeparator />}
				{onDelete && (
					<ContextMenuItem variant="destructive" onSelect={() => onDelete(agent)}>
						<TrashIcon className="size-4" />
						Delete
					</ContextMenuItem>
				)}
			</ContextMenuContent>
		</ContextMenu>
	)
})
