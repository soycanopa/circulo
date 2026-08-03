import { useAtom, useAtomValue, useSetAtom } from "jotai"
import { Download, FileDiff, Loader2, MessageSquarePlus } from "lucide-react"
import { WindowChromeControls } from "@/components/layout/window-chrome-controls"
import { cn } from "@/lib/utils"
import { useCallback, useMemo, useState } from "react"
import { ChatInput } from "@/components/chat/chat-input"
import { MessageList } from "@/components/chat/message-list"
import { CommandPalette } from "@/components/layout/command-palette"
import { AppShell } from "@/components/layout/app-shell"
import { AppSidebar } from "@/components/layout/app-sidebar"
import { OpencodeSetupBanner } from "@/components/onboarding/opencode-setup"
import { OpenProjectModal } from "@/components/project/open-project-modal"
import { SettingsPanel } from "@/components/settings/settings-panel"
import { DiffPanel } from "@/components/tools/diff-panel"
import { useAcpBridge } from "@/hooks/use-acp-bridge"
import { useAppSettings } from "@/hooks/use-app-settings"
import { useAppShortcuts } from "@/hooks/use-app-shortcuts"
import { useBootstrapAgent } from "@/hooks/use-bootstrap"
import { useChatPersistence } from "@/hooks/use-chat-persistence"
import { exportTranscriptMarkdown } from "@/lib/export-transcript"
import {
	agentLabel,
	resolveAgentConnectionStatus,
	resolveAgentDetail,
	type AgentRuntimeState,
} from "@/lib/agent-registry"
import { collectDiffTools } from "@/lib/diff-tools"
import {
	closeSession,
	createSession,
	createWorkspace,
	deleteChatTranscript,
	deleteWorkspace,
	getDefaultChatsPath,
	getAppSettings,
	getHomePath,
	getWorkspacePaths,
	loadChatTranscript,
	loadSession,
	openProject,
	pickDirectory,
	renameChatTranscript,
	setActiveWorkspace,
	setVisibleSession,
} from "@/lib/tauri"

import {
	agentConnectedAtom,
	appSettingsAtom,
	capabilitiesAtom,
	configOptionsAtom,
	diffPanelOpenAtom,
	errorMessageAtom,
	generalChatSessionsAtom,
	generalChatsPathAtom,
	historyViewSessionIdAtom,
	messagesAtom,
	opencodeStatusAtom,
	progressMessageAtom,
	projectChatsByPathAtom,
	projectPathAtom,
	resetWorkspaceUiAtom,
	selectedDiffToolAtom,
	sessionIdAtom,
	sessionStatusAtom,
	sessionsAtom,
	sidebarOpenAtom,
	streamingTextAtom,
	visibleSessionIdAtom,
} from "@/stores/atoms"

export default function App() {
	useAcpBridge()
	useBootstrapAgent()
	useAppSettings()
	const { refreshSessions, refreshPath, refreshAllWorkspaceLists } =
		useChatPersistence()

	const projectPath = useAtomValue(projectPathAtom)
	const sessionId = useAtomValue(sessionIdAtom)
	const historyViewSessionId = useAtomValue(historyViewSessionIdAtom)
	const connected = useAtomValue(agentConnectedAtom)
	const status = useAtomValue(sessionStatusAtom)
	const error = useAtomValue(errorMessageAtom)
	const progress = useAtomValue(progressMessageAtom)
	const opencodeStatus = useAtomValue(opencodeStatusAtom)
	const generalChatsPath = useAtomValue(generalChatsPathAtom)
	const generalChatSessions = useAtomValue(generalChatSessionsAtom)
	const projectChatsByPath = useAtomValue(projectChatsByPathAtom)
	const capabilities = useAtomValue(capabilitiesAtom)
	const appSettings = useAtomValue(appSettingsAtom)
	const messages = useAtomValue(messagesAtom)
	const sessionsMap = useAtomValue(sessionsAtom)
	const visibleSessionId = useAtomValue(visibleSessionIdAtom)
	const liveSessionIds = useMemo(
		() =>
			new Set(
				Object.entries(sessionsMap)
					.filter(
						([sid, state]) =>
							state.promptInFlight && sid !== visibleSessionId,
					)
					.map(([sid]) => sid),
			),
		[sessionsMap, visibleSessionId],
	)
	const diffPanelOpen = useAtomValue(diffPanelOpenAtom)
	const sidebarOpen = useAtomValue(sidebarOpenAtom)
	const setDiffPanelOpen = useSetAtom(diffPanelOpenAtom)
	const setSidebarOpen = useSetAtom(sidebarOpenAtom)
	const setSelectedDiff = useSetAtom(selectedDiffToolAtom)
	const setError = useSetAtom(errorMessageAtom)
	const setStatus = useSetAtom(sessionStatusAtom)
	const setMessages = useSetAtom(messagesAtom)
	const setStreaming = useSetAtom(streamingTextAtom)
	const [, setSessionId] = useAtom(sessionIdAtom)
	const setConnected = useSetAtom(agentConnectedAtom)
	const setProjectPath = useSetAtom(projectPathAtom)
	const setConfig = useSetAtom(configOptionsAtom)
	const setHistoryView = useSetAtom(historyViewSessionIdAtom)
	const setAppSettings = useSetAtom(appSettingsAtom)
	const resetWorkspaceUi = useSetAtom(resetWorkspaceUiAtom)

	const [busy, setBusy] = useState(false)
	const [settingsOpen, setSettingsOpen] = useState(false)
	const [openProjectModalOpen, setOpenProjectModalOpen] = useState(false)
	const [commandPaletteOpen, setCommandPaletteOpen] = useState(false)
	const [agentCommand, setAgentCommand] = useState("opencode acp")
	const [agentId, setAgentId] = useState<string | null>("opencode")

	const agentWarm = connected
	const hasLiveSession = Boolean(sessionId)
	const viewingHistory = Boolean(historyViewSessionId && !sessionId)
	const showChat = hasLiveSession || viewingHistory
	const activeChatId = sessionId ?? historyViewSessionId
	const activeChatTitle = useMemo(() => {
		if (!activeChatId) return "chat"
		const fromGeneral = generalChatSessions.find(
			(c) => c.sessionId === activeChatId,
		)
		if (fromGeneral) return fromGeneral.title
		for (const list of Object.values(projectChatsByPath)) {
			const hit = list.find((c) => c.sessionId === activeChatId)
			if (hit) return hit.title
		}
		return "chat"
	}, [activeChatId, generalChatSessions, projectChatsByPath])

	const handleExportTranscript = useCallback(async () => {
		if (messages.length === 0) return
		try {
			const saved = await exportTranscriptMarkdown(activeChatTitle, messages)
			if (!saved) return
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to export transcript")
		}
	}, [activeChatTitle, messages, setError])

	const toggleDiffPanel = useCallback(() => {
		setDiffPanelOpen((open) => !open)
	}, [setDiffPanelOpen])

	const closeDiffPanel = useCallback(() => {
		setDiffPanelOpen(false)
	}, [setDiffPanelOpen])

	const diffToolCount = useMemo(() => collectDiffTools(messages).length, [messages])

	const agentRuntimes = useMemo((): AgentRuntimeState[] => {
		const available = opencodeStatus?.available ?? true
		const id = agentId ?? "opencode"
		return [
			{
				id,
				label: agentLabel(id),
				command: agentCommand,
				status: resolveAgentConnectionStatus({
					connected,
					statusConnecting: status === "connecting",
					progress,
					available,
				}),
				detail: resolveAgentDetail({
					connected,
					statusConnecting: status === "connecting",
					progress,
					available,
					installHint: opencodeStatus?.installHint ?? null,
					sessionStatus: status,
				}),
			},
		]
	}, [
		agentCommand,
		agentId,
		connected,
		opencodeStatus,
		progress,
		status,
	])

	async function resolveWorkspacePath(rawPath: string): Promise<string> {
		const trimmed = rawPath.trim()
		if (trimmed === "~" || trimmed.startsWith("~/")) {
			const home = await getHomePath()
			if (trimmed === "~") return home
			return `${home}${trimmed.slice(1)}`
		}
		return trimmed
	}

	async function openWorkspacePath(
		path: string,
		options?: { manageBusy?: boolean },
	) {
		const manageBusy = options?.manageBusy ?? true
		const resolved = await resolveWorkspacePath(path)
		const [homePath, base] = await Promise.all([
			getHomePath(),
			Promise.resolve(resolved.split("/").filter(Boolean).pop() ?? resolved),
		])
		const largeRoot =
			base === "Desktop" ||
			base === "Documents" ||
			base === "Downloads" ||
			resolved === homePath

		const isNewWorkspace = projectPath !== resolved

		if (manageBusy) setBusy(true)
		if (isNewWorkspace) {
			resetWorkspaceUi()
			setProjectPath(resolved)
		}
		try {
			const status = await openProject(resolved)
			setProjectPath(status.projectPath)
			setConnected(status.connected)
			setAgentCommand(status.agentCommand)
			setAgentId(status.agentId)
			if (isNewWorkspace) {
				setSessionId(status.sessionId || null)
				setConfig(status.configOptions)
			}
			if (largeRoot) {
				setError(
					"Workspace set. Large folders (Desktop/Home) can slow OpenCode session setup — prefer a repo.",
				)
			}
			setStatus("idle")
			const settings = await getAppSettings()
			setAppSettings(settings)
			setOpenProjectModalOpen(false)
			await refreshAllWorkspaceLists()
			const openedPath = status.projectPath ?? resolved
			await refreshPath(openedPath)
			return openedPath
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to open project")
			setStatus("idle")
			throw err
		} finally {
			if (manageBusy) setBusy(false)
		}
	}

	/** New chat in current context, or in a specific project path. */
	async function handleNewChat(targetProjectPath?: string) {
		setError(null)
		setBusy(true)
		setStatus("connecting")
		setMessages([])
		setStreaming("")
		setHistoryView(null)
		try {
			const desired =
				targetProjectPath ??
				projectPath ??
				(await getDefaultChatsPath())
			if (projectPath !== desired) {
				await openWorkspacePath(desired, { manageBusy: false })
			} else if (!projectPath) {
				await openProject(desired)
			}
			const status = await createSession()
			// Event bridge also sets these; apply return payload so model/mode
			// selects appear even if the event is late or missed.
			if (status.sessionId) setSessionId(status.sessionId)
			if (status.configOptions?.length) setConfig(status.configOptions)
			setStatus("idle")
			await refreshSessions()
			await refreshAllWorkspaceLists()
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to create session")
			setStatus("idle")
		} finally {
			setBusy(false)
		}
	}

	function handleOpenProject() {
		setError(null)
		setOpenProjectModalOpen(true)
	}

	async function handleSelectGeneralChats() {
		setError(null)
		try {
			const path = generalChatsPath ?? (await getDefaultChatsPath())
			await openWorkspacePath(path)
		} catch {
			// Error banner already set in openWorkspacePath.
		}
	}

	async function handleAddWorkspace() {
		setError(null)
		setBusy(true)
		try {
			const settings = await createWorkspace()
			setAppSettings(settings)
			const id = settings.activeWorkspaceId
			if (!id) return
			const paths = await getWorkspacePaths(id)
			await openWorkspacePath(paths.chatsPath, { manageBusy: false })
			await refreshAllWorkspaceLists()
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to create workspace")
		} finally {
			setBusy(false)
		}
	}

	async function handleSelectWorkspace(workspaceId: string) {
		if (workspaceId === appSettings?.activeWorkspaceId) return
		setError(null)
		setBusy(true)
		try {
			const settings = await setActiveWorkspace(workspaceId)
			setAppSettings(settings)
			const paths = await getWorkspacePaths(workspaceId)
			await openWorkspacePath(paths.entryPath, { manageBusy: false })
			await refreshAllWorkspaceLists()
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to switch workspace")
		} finally {
			setBusy(false)
		}
	}

	async function handleDeleteWorkspace(workspaceId: string) {
		setError(null)
		setBusy(true)
		try {
			const settings = await deleteWorkspace(workspaceId)
			setAppSettings(settings)
			const nextId = settings.activeWorkspaceId
			if (nextId) {
				const paths = await getWorkspacePaths(nextId)
				await openWorkspacePath(paths.entryPath, { manageBusy: false })
			}
			await refreshAllWorkspaceLists()
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to remove workspace")
		} finally {
			setBusy(false)
		}
	}

	useAppShortcuts({
		onNewChat: () => {
			if (!busy) void handleNewChat()
		},
		onOpenProject: () => {
			if (!busy) handleOpenProject()
		},
		onOpenSettings: () => setSettingsOpen(true),
		onOpenCommandPalette: () => setCommandPaletteOpen(true),
		onExportTranscript: () => {
			void handleExportTranscript()
		},
	})

	const commandItems = useMemo(
		() => [
			{
				id: "new-chat",
				label: "New Chat",
				shortcut: "⌘N",
				onSelect: () => {
					void handleNewChat()
				},
			},
			{
				id: "open-project",
				label: "Open Project",
				onSelect: () => handleOpenProject(),
			},
			{
				id: "settings",
				label: "Settings",
				onSelect: () => setSettingsOpen(true),
			},
			{
				id: "diff-panel",
				label: diffPanelOpen ? "Close Diff Panel" : "Open Diff Panel",
				onSelect: () => toggleDiffPanel(),
			},
			...(messages.length > 0
				? [
						{
							id: "export",
							label: "Export Transcript",
							shortcut: "⌘⇧E",
							onSelect: () => void handleExportTranscript(),
						},
					]
				: []),
		],
		[
			diffPanelOpen,
			handleExportTranscript,
			handleNewChat,
			messages.length,
			toggleDiffPanel,
		],
	)

	async function handleRenameChat(
		targetSessionId: string,
		ownerPath: string,
		title: string,
	) {
		try {
			await renameChatTranscript(ownerPath, targetSessionId, title)
			await refreshPath(ownerPath)
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to rename chat")
		}
	}

	async function handleSelectProject(path: string) {
		setError(null)
		try {
			await openWorkspacePath(path)
		} catch {
			// Error banner already set in openWorkspacePath.
		}
	}

	async function handleOpenChat(targetSessionId: string, ownerPath: string) {
		setError(null)

		const sameWorkspace = projectPath === ownerPath
		if (sameWorkspace && targetSessionId === sessionId) {
			setHistoryView(null)
			return
		}

		// If the session is already alive on the same agent process (background run),
		// swap the visible session instead of resuming from disk. The reducer mirrors
		// the active buffer into messagesAtom/streamingTextAtom automatically.
		if (sameWorkspace && sessionsMap[targetSessionId]) {
			try {
				await setVisibleSession(targetSessionId)
				setHistoryView(null)
				return
			} catch (err) {
				setError(
					err instanceof Error ? err.message : "Failed to swap session",
				)
				return
			}
		}

		setBusy(true)
		try {
			if (!sameWorkspace) {
				await openWorkspacePath(ownerPath, { manageBusy: false })
			}

			const transcript = await loadChatTranscript(ownerPath, targetSessionId)
			setMessages(transcript.messages)
			setStreaming("")
			setStatus("connecting")

			// Only try live resume when we stayed on the same agent workspace.
			if (capabilities?.loadSession && sameWorkspace) {
				try {
					await loadSession(targetSessionId)
					setHistoryView(null)
					setStatus("idle")
					return
				} catch (err) {
					setSessionId(null)
					setHistoryView(targetSessionId)
					setStatus("idle")
					const detail =
						err instanceof Error ? err.message : "Could not resume session"
					setError(`${detail} — viewing saved transcript only.`)
					return
				}
			}

			// After workspace switch (or no loadSession): saved transcript only.
			setSessionId(null)
			setHistoryView(targetSessionId)
			setStatus("idle")
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to load chat")
			setStatus("idle")
		} finally {
			setBusy(false)
		}
	}

	async function handleDeleteChat(
		targetSessionId: string,
		ownerPath: string,
	) {
		setError(null)
		setBusy(true)
		try {
			const sameWorkspace = projectPath === ownerPath
			if (sameWorkspace && sessionId === targetSessionId) {
				try {
					await closeSession(targetSessionId)
				} catch {
					// Still remove local transcript if agent close fails.
				}
				setSessionId(null)
				setMessages([])
				setStreaming("")
			}
			if (sameWorkspace && historyViewSessionId === targetSessionId) {
				setHistoryView(null)
				setMessages([])
			}
			await deleteChatTranscript(ownerPath, targetSessionId)
			await refreshPath(ownerPath)
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to delete chat")
		} finally {
			setBusy(false)
		}
	}

	const statusLabel = viewingHistory
		? "History — New Chat to continue"
		: status === "generating"
			? "Streaming…"
			: status === "awaiting_permission"
				? "Waiting for permission…"
				: status === "connecting"
					? progress || "Opening session…"
					: hasLiveSession
						? "Ready"
						: agentWarm
							? "Ready — New Chat"
							: "Ready"

	return (
		<>
			<AppShell
				sidebarOpen={sidebarOpen}
				panelOpen={diffPanelOpen}
				panel={
					<DiffPanel
						onClose={() => {
							closeDiffPanel()
							setSelectedDiff(null)
						}}
					/>
				}
				sidebar={
			<AppSidebar
				sessionId={sessionId}
				historyViewSessionId={historyViewSessionId}
				agentRuntimes={agentRuntimes}
				busy={busy}
				generalChatsPath={generalChatsPath}
				generalChats={generalChatSessions}
				projectChatsByPath={projectChatsByPath}
				workspaces={appSettings?.workspaces ?? []}
				activeWorkspaceId={appSettings?.activeWorkspaceId ?? null}
				currentProjectPath={projectPath}
				liveSessionIds={liveSessionIds}
				onNewChat={() => void handleNewChat()}
				onNewChatInProject={(path) => void handleNewChat(path)}
						onOpenProject={() => handleOpenProject()}
						onOpenSettings={() => setSettingsOpen(true)}
						onOpenChat={(id, ownerPath) => void handleOpenChat(id, ownerPath)}
						onRenameChat={(id, ownerPath, title) =>
							void handleRenameChat(id, ownerPath, title)
						}
						onDeleteChat={(id, ownerPath) =>
							void handleDeleteChat(id, ownerPath)
						}
						onSelectProject={(path) => void handleSelectProject(path)}
						onSelectGeneralChats={() => void handleSelectGeneralChats()}
						onAddWorkspace={() => void handleAddWorkspace()}
						onSelectWorkspace={(id) => void handleSelectWorkspace(id)}
						onDeleteWorkspace={(id) => void handleDeleteWorkspace(id)}
						onHideSidebar={() => setSidebarOpen(false)}
					/>
				}
			>
				<div
					className="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-border pb-0.5"
					data-tauri-drag-region="deep"
				>
					<div className="flex min-w-0 flex-1 items-center">
						{!sidebarOpen ? (
							<div className="flex shrink-0 items-center gap-1.5">
								<WindowChromeControls
									sidebarOpen={false}
									layout="inline"
									onToggleSidebar={() => setSidebarOpen(true)}
								/>
								<button
									type="button"
									onClick={() => void handleNewChat()}
									disabled={busy}
									className="inline-flex items-center gap-1 rounded-md border border-border px-2 py-1 text-[11px] text-muted transition hover:bg-white/5 hover:text-fg disabled:opacity-40"
									title="New chat (⌘N)"
									data-tauri-drag-region="false"
								>
									<MessageSquarePlus className="size-3.5" />
									New chat
								</button>
							</div>
						) : null}
						<div
							className={cn(
								"min-w-0 flex-1 truncate text-xs text-muted",
								sidebarOpen ? "px-4" : "pl-2 pr-4",
							)}
						>
							{statusLabel}
						</div>
					</div>
					<div className="flex shrink-0 items-center gap-2 pr-4">
						<button
							type="button"
							onClick={toggleDiffPanel}
							className={`inline-flex items-center gap-1 rounded-md border px-2 py-1 text-[11px] transition ${
								diffPanelOpen
									? "border-sky-500/40 bg-sky-500/10 text-sky-200"
									: "border-border text-muted hover:bg-white/5 hover:text-fg"
							}`}
							title="Toggle diff panel"
							data-tauri-drag-region="false"
						>
							<FileDiff className="size-3.5" />
							Diff
							{diffToolCount > 0 ? (
								<span className="rounded bg-white/10 px-1 text-[10px]">
									{diffToolCount}
								</span>
							) : null}
						</button>
						{showChat && messages.length > 0 ? (
							<button
								type="button"
								onClick={() => void handleExportTranscript()}
								className="inline-flex items-center gap-1 rounded-md border border-border px-2 py-1 text-[11px] text-muted transition hover:bg-white/5 hover:text-fg"
								title="Export transcript (⌘⇧E)"
								data-tauri-drag-region="false"
							>
								<Download className="size-3.5" />
								Export
							</button>
						) : null}
					</div>
				</div>
			{opencodeStatus && !opencodeStatus.available ? (
				<OpencodeSetupBanner status={opencodeStatus} />
			) : null}

			{error ? (
				<div className="mx-4 mt-3 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200">
					{error}
				</div>
			) : null}

			{viewingHistory ? (
				<div className="mx-4 mt-3 rounded-md border border-border bg-white/5 px-3 py-2 text-xs text-muted">
					Viewing saved history
					{capabilities?.loadSession
						? " — could not resume on the agent."
						: " — session resume not supported by this agent."}{" "}
					Start a{" "}
					<button
						type="button"
						onClick={() => void handleNewChat()}
						className="text-fg underline-offset-2 hover:underline"
					>
						New Chat
					</button>{" "}
					to talk to the agent again.
				</div>
			) : null}

			{!showChat ? (
				<div className="flex flex-1 flex-col items-center justify-center gap-3 px-8 text-center">
					<p className="text-lg font-medium tracking-tight">Circulo</p>
					<p className="max-w-md text-sm text-muted">
						{agentWarm
							? "Start a conversation with New Chat."
							: "App is ready. OpenCode warms in the background — New Chat waits only if you click before it's up."}
					</p>
					<button
						type="button"
						onClick={() => void handleNewChat()}
						disabled={busy}
						className="mt-2 inline-flex items-center gap-2 rounded-md bg-white/10 px-3 py-1.5 text-sm text-fg transition hover:bg-white/15 disabled:opacity-40"
					>
						{busy ? (
							<Loader2 className="size-4 animate-spin" />
						) : (
							<MessageSquarePlus className="size-4" />
						)}
						New chat
					</button>
				</div>
			) : (
				<MessageList />
			)}

			<ChatInput />
			<SettingsPanel
				open={settingsOpen}
				onClose={() => setSettingsOpen(false)}
				agentCommand={agentCommand}
			/>
			<OpenProjectModal
				open={openProjectModalOpen}
				busy={busy}
				recentProjects={appSettings?.recentProjects ?? []}
				currentProjectPath={projectPath}
				onClose={() => setOpenProjectModalOpen(false)}
				onOpenPath={async (path) => {
					await openWorkspacePath(path)
				}}
				onBrowseFinder={pickDirectory}
			/>
			</AppShell>
			<CommandPalette
				open={commandPaletteOpen}
				items={commandItems}
				onClose={() => setCommandPaletteOpen(false)}
			/>
		</>
	)
}
