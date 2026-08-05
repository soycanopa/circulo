import { getDefaultStore, useAtomValue, useSetAtom } from "jotai"
import { Download, FileDiff, MessageSquarePlus, Terminal } from "lucide-react"
import { WindowChromeControls } from "@/components/layout/window-chrome-controls"
import { cn } from "@/lib/utils"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { ChatInput } from "@/components/chat/chat-input"
import { MessageList } from "@/components/chat/message-list"
import { CommandPalette } from "@/components/layout/command-palette"
import { AppShell } from "@/components/layout/app-shell"
import { AppSidebar } from "@/components/layout/app-sidebar"
import { OpencodeSetupBanner } from "@/components/onboarding/opencode-setup"
import { OpenProjectModal } from "@/components/project/open-project-modal"
import { SettingsPanel } from "@/components/settings/settings-panel"
import { DiffPanel } from "@/components/tools/diff-panel"
import { TerminalDrawer } from "@/components/terminal/terminal-drawer"
import { useAcpBridge } from "@/hooks/use-acp-bridge"
import { useTerminalBridge } from "@/hooks/use-terminal-bridge"
import { useAppSettings } from "@/hooks/use-app-settings"
import { useAppShortcuts } from "@/hooks/use-app-shortcuts"
import { useBootstrapAgent } from "@/hooks/use-bootstrap"
import { useAutomations } from "@/hooks/use-automations"
import { useChatPersistence } from "@/hooks/use-chat-persistence"
import { reconcileSessionFromProjectStatus } from "@/hooks/session-reconcile"
import type { AppSettings } from "@/types/acp"
import { exportTranscriptMarkdown } from "@/lib/export-transcript"
import {
	agentLabel,
	resolveAgentConnectionStatus,
	resolveAgentDetail,
	type AgentRuntimeState,
} from "@/lib/agent-registry"
import { collectDiffTools } from "@/lib/diff-tools"
import { collectTerminalTools } from "@/lib/terminal-tools"
import {
	closeSession,
	createSession,
	createWorkspace,
	deleteChatTranscript,
	deleteAutomation,
	deleteWorkspace,
	getDefaultChatsPath,
	getAppSettings,
	getHomePath,
	getProjectStatus,
	getWorkspacePaths,
	loadChatTranscript,
	loadSession,
	openInEditor,
	openProject,
	pickDirectory,
	renameChatTranscript,
	removeProjectFromWorkspace,
	seedChatTranscript,
	sendPrompt,
	setActiveWorkspace,
	setAllowedTool,
	setPreferredAgent,
	setVisibleSession,
} from "@/lib/tauri"

import {
	activeSessionIdAtom,
	agentConnectedAtom,
	appSettingsAtom,
	capabilitiesAtom,
	diffPanelOpenAtom,
	errorMessageAtom,
	generalChatSessionsAtom,
	generalChatsPathAtom,
	historyMessagesAtom,
	historyViewSessionIdAtom,
	opencodeStatusAtom,
	progressMessageAtom,
	projectChatsByPathAtom,
	projectPathAtom,
	resetWorkspaceUiAtom,
	selectedDiffToolAtom,
	sessionStatusAtom,
	sessionsAtom,
	sidebarOpenAtom,
	terminalDrawerOpenAtom,
	terminalsAtom,
	TERMINAL_DRAWER_HEIGHT_DEFAULT,
	userTerminalTabsAtom,
	visibleMessagesAtom,
	visibleSessionStatusAtom,
} from "@/stores/atoms"

export default function App() {
	useAcpBridge()
	useTerminalBridge()
	useBootstrapAgent()
	useAppSettings()
	const { refreshSessions, refreshPath, refreshAllWorkspaceLists } =
		useChatPersistence()
	const { automations, refresh: refreshAutomations } = useAutomations()

	const projectPath = useAtomValue(projectPathAtom)
	const sessionId = useAtomValue(activeSessionIdAtom)
	const historyViewSessionId = useAtomValue(historyViewSessionIdAtom)
	const connected = useAtomValue(agentConnectedAtom)
	const status = useAtomValue(visibleSessionStatusAtom)
	const error = useAtomValue(errorMessageAtom)
	const progress = useAtomValue(progressMessageAtom)
	const opencodeStatus = useAtomValue(opencodeStatusAtom)
	const generalChatsPath = useAtomValue(generalChatsPathAtom)
	const generalChatSessions = useAtomValue(generalChatSessionsAtom)
	const projectChatsByPath = useAtomValue(projectChatsByPathAtom)
	const capabilities = useAtomValue(capabilitiesAtom)
	const appSettings = useAtomValue(appSettingsAtom)
	const messages = useAtomValue(visibleMessagesAtom)
	const sessionsMap = useAtomValue(sessionsAtom)
	const liveSessionIds = useMemo(
		() =>
			new Set(
				Object.entries(sessionsMap)
					.filter(
						([, state]) =>
							state.promptInFlight || state.status === "generating",
					)
					.map(([sid]) => sid),
			),
		[sessionsMap],
	)
	const diffPanelOpen = useAtomValue(diffPanelOpenAtom)
	const terminalDrawerOpen = useAtomValue(terminalDrawerOpenAtom)
	const terminals = useAtomValue(terminalsAtom)
	const userTerminalTabs = useAtomValue(userTerminalTabsAtom)
	const sidebarOpen = useAtomValue(sidebarOpenAtom)
	const setDiffPanelOpen = useSetAtom(diffPanelOpenAtom)
	const setTerminalDrawerOpen = useSetAtom(terminalDrawerOpenAtom)
	const setSidebarOpen = useSetAtom(sidebarOpenAtom)
	const setSelectedDiff = useSetAtom(selectedDiffToolAtom)
	const setError = useSetAtom(errorMessageAtom)
	const setStatus = useSetAtom(sessionStatusAtom)
	const setConnected = useSetAtom(agentConnectedAtom)
	const setProjectPath = useSetAtom(projectPathAtom)
	const setHistoryView = useSetAtom(historyViewSessionIdAtom)
	const setHistoryMessages = useSetAtom(historyMessagesAtom)
	const setAppSettings = useSetAtom(appSettingsAtom)
	const resetWorkspaceUi = useSetAtom(resetWorkspaceUiAtom)

	const [busy, setBusy] = useState(false)
	const [settingsOpen, setSettingsOpen] = useState(false)
	const [openProjectModalOpen, setOpenProjectModalOpen] = useState(false)
	const [commandPaletteOpen, setCommandPaletteOpen] = useState(false)
	const [terminalMounted, setTerminalMounted] = useState(false)
	const [agentCommand, setAgentCommand] = useState("opencode acp")
	const [agentId, setAgentId] = useState<string | null>("opencode")

	const hasLiveSession = Boolean(sessionId)
	const browsingSavedChat = Boolean(
		historyViewSessionId && historyViewSessionId !== sessionId,
	)
	const showChat = hasLiveSession || browsingSavedChat || connected
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

	// Seed a sidebar row only after the user has sent at least one message.
	const seededSessionsRef = useRef(new Set<string>())
	useEffect(() => {
		if (!sessionId || !projectPath) return
		const liveMessages = sessionsMap[sessionId]?.messages ?? []
		if (liveMessages.length === 0) return
		const seedKey = `${projectPath}::${sessionId}`
		if (seededSessionsRef.current.has(seedKey)) return
		const known = generalChatSessions.some(
			(c) => c.sessionId === sessionId,
		)
		const knownProject = Object.values(projectChatsByPath).some((list) =>
			list.some((c) => c.sessionId === sessionId),
		)
		const knownLive = Boolean(sessionsMap[sessionId]?.messages.length)
		if (known || knownProject || knownLive) {
			seededSessionsRef.current.add(seedKey)
			return
		}
		seededSessionsRef.current.add(seedKey)
		void seedChatTranscript(projectPath, sessionId, "New chat").catch(() => {
			// Best-effort seed; the chat already exists if this fails.
		})
	}, [
		sessionId,
		projectPath,
		generalChatSessions,
		projectChatsByPath,
		sessionsMap,
	])

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

	const handleOpenInEditor = useCallback(
		(editor: "vscode" | "cursor" | "terminal") => {
			if (!projectPath) return
			setError(null)
			void openInEditor(editor, projectPath).catch((err: unknown) => {
				setError(
					err instanceof Error ? err.message : "Failed to open in editor",
				)
			})
		},
		[projectPath, setError],
	)

	const toggleTerminalDrawer = useCallback(() => {
		setTerminalDrawerOpen((open) => {
			if (!open) setTerminalMounted(true)
			return !open
		})
	}, [setTerminalDrawerOpen])

	useEffect(() => {
		if (terminalDrawerOpen) setTerminalMounted(true)
	}, [terminalDrawerOpen])

	const closeTerminalDrawer = useCallback(() => {
		setTerminalDrawerOpen(false)
	}, [setTerminalDrawerOpen])

	const diffToolCount = useMemo(() => collectDiffTools(messages).length, [messages])
	const terminalCount = useMemo(
		() =>
			Math.max(
				userTerminalTabs.length,
				collectTerminalTools(messages).length,
				Object.keys(terminals).length,
			),
		[messages, terminals, userTerminalTabs.length],
	)

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

	async function handleAgentChange(agentId: string) {
		setError(null)
		const priorSid = getDefaultStore().get(activeSessionIdAtom)
		const priorMessages = getDefaultStore().get(visibleMessagesAtom)
		if (priorSid && projectPath && priorMessages.length === 0) {
			try {
				await deleteChatTranscript(projectPath, priorSid)
			} catch {
				// Best-effort: drop empty sidebar row when switching agent.
			}
		}
		try {
			const settings = await setPreferredAgent(agentId)
			setAppSettings(settings)
			setAgentId(agentId)
			if (!projectPath) return

			setStatus("connecting")
			resetWorkspaceUi()
			getDefaultStore().set(
				progressMessageAtom,
				`Conectando ${agentLabel(agentId)}…`,
			)
			const status = await openProject(projectPath, agentId)
			setProjectPath(status.projectPath)
			setConnected(status.connected)
			setAgentCommand(status.agentCommand)
			setAgentId(status.agentId)
			reconcileSessionFromProjectStatus(getDefaultStore(), status)
			setStatus("idle")
			void refreshAllWorkspaceLists()
			void refreshPath(projectPath)
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to switch agent")
			setStatus("idle")
		}
	}

	async function handleEnabledAgentsChange(settings: AppSettings) {
		setAppSettings(settings)
		const nextPreferred = settings.preferredAgentId ?? null
		setAgentId(nextPreferred)
		if (projectPath && nextPreferred && nextPreferred !== agentId) {
			await handleAgentChange(nextPreferred)
		}
	}

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
			// Reset only when the workspace really changes so background sessions
			// from the previous workspace don't get wiped out mid-prompt.
			resetWorkspaceUi()
			setProjectPath(resolved)
		} else {
			setProjectPath(resolved)
		}
		try {
			const status = await openProject(resolved)
			setProjectPath(status.projectPath)
			setConnected(status.connected)
			setAgentCommand(status.agentCommand)
			setAgentId(status.agentId)
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
		setHistoryView(null)
		setHistoryMessages([])
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
			// C9: refresh sidebar before creating the session so the new chat
			// appears in the workspace list as soon as the reducer emits
			// session_ready. The reducer is the source of truth for the new
			// active session id; we do not write activeSessionIdAtom here.
			const priorSid = getDefaultStore().get(activeSessionIdAtom)
			const priorMessages = getDefaultStore().get(visibleMessagesAtom)
			if (priorSid && desired && priorMessages.length === 0) {
				try {
					await deleteChatTranscript(desired, priorSid)
				} catch {
					// Rust closes the pristine session; drop orphan sidebar row.
				}
			}
			await refreshAllWorkspaceLists()
			const status = await createSession()
			reconcileSessionFromProjectStatus(getDefaultStore(), status)
			await refreshSessions()
			await refreshAllWorkspaceLists()
		} catch (err) {
			try {
				const status = await getProjectStatus()
				if (status.sessionId) {
					reconcileSessionFromProjectStatus(getDefaultStore(), status)
				}
			} catch {
				// Best-effort recovery when create failed but Rust bound a session.
			}
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

	async function handleRemoveProject(path: string) {
		setError(null)
		setBusy(true)
		try {
			const settings = await removeProjectFromWorkspace(path)
			setAppSettings(settings)
			const wasActive = projectPath === path
			await refreshAllWorkspaceLists()
			if (wasActive) {
				await handleSelectGeneralChats()
			}
		} catch (err) {
			setError(
				err instanceof Error ? err.message : "Failed to remove project",
			)
		} finally {
			setBusy(false)
		}
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

	const handleRunAutomation = useCallback(
		async (prompt: string) => {
			if (!sessionId) {
				setError("Start a New Chat before running an automation")
				return
			}
			setError(null)
			try {
				await sendPrompt(prompt, [])
			} catch (err) {
				setError(err instanceof Error ? err.message : "Failed to run automation")
			}
		},
		[sessionId, setError],
	)

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
			...(projectPath
				? [
						{
							id: "open-in-vscode",
							label: "Open in VS Code",
							onSelect: () => void handleOpenInEditor("vscode"),
						},
						{
							id: "open-in-cursor",
							label: "Open in Cursor",
							onSelect: () => void handleOpenInEditor("cursor"),
						},
						{
							id: "open-in-terminal",
							label: "Open in Terminal",
							onSelect: () => void handleOpenInEditor("terminal"),
						},
					]
				: []),
			...automations.map((automation) => ({
				id: `automation-${automation.id}`,
				label: `Run: ${automation.title}`,
				onSelect: () => void handleRunAutomation(automation.prompt),
			})),
			{
				id: "diff-panel",
				label: diffPanelOpen ? "Close Diff Panel" : "Open Diff Panel",
				onSelect: () => toggleDiffPanel(),
			},
			{
				id: "terminal-drawer",
				label: terminalDrawerOpen ? "Close Terminal" : "Open Terminal",
				onSelect: () => toggleTerminalDrawer(),
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
			automations,
			diffPanelOpen,
			handleExportTranscript,
			handleNewChat,
			handleOpenInEditor,
			handleRunAutomation,
			messages.length,
			projectPath,
			terminalDrawerOpen,
			toggleDiffPanel,
			toggleTerminalDrawer,
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

		async function showSavedTranscript() {
			const transcript = await loadChatTranscript(ownerPath, targetSessionId)
			setHistoryMessages(transcript.messages)
			setHistoryView(targetSessionId)
			if (sameWorkspace) {
				try {
					await setVisibleSession(null)
				} catch {
					// Best-effort: clear the live binding while browsing disk history.
				}
			}
		}

		// If the session is already alive on the same agent process (background run),
		// swap the visible session instead of resuming from disk. The reducer mirrors
		// the active buffer into messagesAtom/streamingTextAtom automatically.
		if (sameWorkspace && sessionsMap[targetSessionId]) {
			try {
				const status = await setVisibleSession(targetSessionId)
				reconcileSessionFromProjectStatus(getDefaultStore(), status)
				setHistoryView(null)
				setHistoryMessages([])
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

			// Live resume when we stayed on the same agent workspace and the agent supports it.
			if (capabilities?.loadSession && sameWorkspace) {
				try {
					const status = await loadSession(targetSessionId)
					reconcileSessionFromProjectStatus(getDefaultStore(), status)
					setHistoryView(null)
					setHistoryMessages([])
					setStatus("idle")
					return
				} catch {
					await showSavedTranscript()
					setStatus("idle")
					return
				}
			}

			await showSavedTranscript()
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
			}
			await deleteChatTranscript(ownerPath, targetSessionId)
			await refreshPath(ownerPath)
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to delete chat")
		} finally {
			setBusy(false)
		}
	}

	const statusLabel =
		status === "generating"
			? "Streaming…"
			: status === "awaiting_permission"
				? "Waiting for permission…"
				: status === "connecting"
					? progress || "Opening session…"
					: "Ready"

	const shellTransition =
		"transition-[height] duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]"

	return (
		<>
			<AppShell
				sidebarOpen={sidebarOpen}
				panelOpen={diffPanelOpen}
				panel={
					<DiffPanel
						projectPath={projectPath}
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
						onRemoveProject={(path) => void handleRemoveProject(path)}
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
							onClick={toggleTerminalDrawer}
							className={`relative inline-flex items-center justify-center rounded-md border p-1.5 transition ${
								terminalDrawerOpen
									? "border-emerald-500/40 bg-emerald-500/10 text-emerald-200"
									: "border-border text-muted hover:bg-white/5 hover:text-fg"
							}`}
							title="Toggle terminal"
							aria-label="Toggle terminal"
							data-tauri-drag-region="false"
						>
							<Terminal className="size-3.5" />
							{terminalCount > 0 ? (
								<span className="absolute -right-1 -top-1 flex size-3.5 items-center justify-center rounded-full bg-white/15 text-[9px] leading-none text-fg">
									{terminalCount > 9 ? "9+" : terminalCount}
								</span>
							) : null}
						</button>
						<button
							type="button"
							onClick={toggleDiffPanel}
							className={`relative inline-flex items-center justify-center rounded-md border p-1.5 transition ${
								diffPanelOpen
									? "border-sky-500/40 bg-sky-500/10 text-sky-200"
									: "border-border text-muted hover:bg-white/5 hover:text-fg"
							}`}
							title="Toggle diff panel"
							aria-label="Toggle diff panel"
							data-tauri-drag-region="false"
						>
							<FileDiff className="size-3.5" />
							{diffToolCount > 0 ? (
								<span className="absolute -right-1 -top-1 flex size-3.5 items-center justify-center rounded-full bg-white/15 text-[9px] leading-none text-fg">
									{diffToolCount > 9 ? "9+" : diffToolCount}
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

			<div className="flex min-h-0 flex-1 flex-col">
				{showChat ? <MessageList /> : null}
			</div>

			{appSettings ? (
				<ChatInput
					enabledAgentIds={appSettings.enabledAgentIds ?? []}
					preferredAgentId={appSettings.preferredAgentId ?? agentId}
					onAgentChange={handleAgentChange}
					onNewChat={() => void handleNewChat()}
				/>
			) : (
				<ChatInput onAgentChange={undefined} onNewChat={() => void handleNewChat()} />
			)}

			<div
				className={cn(
					"terminal-panel-shell shrink-0 overflow-hidden border-t border-border",
					shellTransition,
				)}
				style={{
					height: terminalDrawerOpen ? TERMINAL_DRAWER_HEIGHT_DEFAULT : 0,
				}}
			>
				<div
					className={cn(
						"flex flex-col",
						!terminalDrawerOpen && "pointer-events-none",
					)}
					style={{ height: TERMINAL_DRAWER_HEIGHT_DEFAULT }}
					aria-hidden={!terminalDrawerOpen}
				>
					{terminalMounted ? (
						<TerminalDrawer
							projectPath={projectPath}
							onClose={closeTerminalDrawer}
						/>
					) : null}
				</div>
			</div>
			<SettingsPanel
				open={settingsOpen}
				onClose={() => setSettingsOpen(false)}
				agentCommand={agentCommand}
				preferredAgentId={appSettings?.preferredAgentId}
				enabledAgentIds={
					appSettings?.enabledAgentIds ?? ["opencode", "cursor-agent"]
				}
				allowedToolPatterns={appSettings?.allowedToolPatterns ?? []}
				onPreferredAgentChange={(agentId) => void handleAgentChange(agentId)}
				onEnabledAgentsChange={(settings) =>
					void handleEnabledAgentsChange(settings)
				}
				onSetAllowedTool={async (pattern, enabled) => {
					const settings = await setAllowedTool(pattern, enabled)
					setAppSettings(settings)
				}}
				automations={automations}
				onAutomationsChange={() => void refreshAutomations()}
				onDeleteAutomation={async (id) => {
					await deleteAutomation(id)
					await refreshAutomations()
				}}
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
