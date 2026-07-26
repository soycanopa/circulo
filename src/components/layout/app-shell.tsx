import { useAtom, useSetAtom } from "jotai"
import { useEffect, useState } from "react"
import { SessionTitle } from "@/components/chat/session-title"
import { AppSidebar } from "@/components/layout/app-sidebar"
import { SidebarLayout } from "@/components/layout/sidebar-layout"
import { ChatView } from "@/components/chat/chat-view"
import { SettingsSidebar } from "@/components/settings/settings-sidebar"
import { SettingsTitle } from "@/components/settings/settings-title"
import { SettingsView } from "@/components/settings/settings-view"
import { bootstrapAppStatus, setLastProjectPath } from "@/lib/app-bootstrap"
import { getAppSettings } from "@/lib/app-settings"
import type { AgentProviderId } from "@/lib/agent-providers"
import { normalizeSessionId } from "@/lib/session-id"
import { addRecentProject } from "@/lib/recent-projects"
import { isGeneralChatProject } from "@/lib/project-display"
import type { OpenProjectOptions } from "@/lib/open-project"
import { closeProject, getProjectStatus, loadSession, openProject } from "@/lib/tauri"
import { waitForAgentReady } from "@/lib/wait-for-agent-ready"
import {
	activeAgentIdAtom,
	activeSessionIdAtom,
	agentCapabilitiesAtom,
	agentConnectedAtom,
	configOptionsAtom,
	diffPanelOpenAtom,
	messagesAtom,
	projectPathAtom,
	replayingHistoryAtom,
	sessionStatusAtom,
	sessionsAtom,
	settingsOpenAtom,
	streamingTextAtom,
	terminalOpenAtom,
} from "@/stores/atoms"

function applyProjectStatus(
	status: Awaited<ReturnType<typeof openProject>>,
	handlers: {
		setAgentConnected: (value: boolean) => void
		setProjectPath: (value: string | null) => void
		setSessions: (value: typeof status.sessions) => void
		setActiveSessionId: (value: string | null) => void
		setCapabilities: (value: typeof status.capabilities) => void
		setActiveAgentId: (value: AgentProviderId) => void
	},
	options?: { skipActiveSession?: boolean },
) {
	handlers.setAgentConnected(status.connected)
	handlers.setProjectPath(status.projectPath)
	handlers.setSessions(status.sessions)
	if (!options?.skipActiveSession) {
		const sessionId = normalizeSessionId(status.activeSessionId ?? status.sessionId)
		handlers.setActiveSessionId(sessionId)
	}
	if (status.agentId) handlers.setActiveAgentId(status.agentId as AgentProviderId)
	handlers.setCapabilities(status.capabilities)
}

export function AppShell() {
	const [projectPath, setProjectPath] = useAtom(projectPathAtom)
	const [agentConnected, setAgentConnected] = useAtom(agentConnectedAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const setActiveSessionId = useSetAtom(activeSessionIdAtom)
	const setCapabilities = useSetAtom(agentCapabilitiesAtom)
	const setActiveAgentId = useSetAtom(activeAgentIdAtom)
	const setMessages = useSetAtom(messagesAtom)
	const setStreamingText = useSetAtom(streamingTextAtom)
	const setConfigOptions = useSetAtom(configOptionsAtom)
	const setReplayingHistory = useSetAtom(replayingHistoryAtom)
	const [sessionStatus] = useAtom(sessionStatusAtom)
	const [settingsOpen, setSettingsOpen] = useAtom(settingsOpenAtom)
	const setDiffPanelOpen = useSetAtom(diffPanelOpenAtom)
	const setTerminalOpen = useSetAtom(terminalOpenAtom)
	const [loading, setLoading] = useState(false)

	useEffect(() => {
		if (!settingsOpen) return
		setDiffPanelOpen(false)
		setTerminalOpen(false)
	}, [settingsOpen, setDiffPanelOpen, setTerminalOpen])

	useEffect(() => {
		if (!settingsOpen) return

		function onKeyDown(event: KeyboardEvent) {
			if (event.key === "Escape") {
				event.preventDefault()
				setSettingsOpen(false)
			}
		}

		window.addEventListener("keydown", onKeyDown)
		return () => window.removeEventListener("keydown", onKeyDown)
	}, [settingsOpen, setSettingsOpen])

	useEffect(() => {
		let cancelled = false

		async function bootstrap() {
			try {
				const status = await bootstrapAppStatus()
				if (cancelled) return
				applyProjectStatus(status, {
					setAgentConnected,
					setProjectPath,
					setSessions,
					setActiveSessionId,
					setCapabilities,
					setActiveAgentId,
				})

				if (!status.connected) {
					try {
						await waitForAgentReady()
					} catch {
						return
					}
				}

				if (cancelled) return

				const latest = await getProjectStatus()
				if (cancelled) return

				if (
					latest.connected &&
					latest.sessions.length === 0 &&
					isGeneralChatProject(latest.projectPath)
				) {
					const closed = await closeProject()
					if (cancelled) return
					applyProjectStatus(closed, {
						setAgentConnected,
						setProjectPath,
						setSessions,
						setActiveSessionId,
						setCapabilities,
						setActiveAgentId,
					})
					return
				}

				applyProjectStatus(latest, {
					setAgentConnected,
					setProjectPath,
					setSessions,
					setActiveSessionId,
					setCapabilities,
					setActiveAgentId,
				})

				const sessionId = normalizeSessionId(
					latest.activeSessionId ?? latest.sessionId,
				)
				const hasTrackedSession = latest.sessions.some(
					(session) => session.sessionId === sessionId,
				)
				if (!sessionId || !hasTrackedSession) return

				setReplayingHistory(true)
				await loadSession(sessionId)
			} catch {
				if (!cancelled) setAgentConnected(false)
			}
		}

		void bootstrap()
		return () => {
			cancelled = true
		}
	}, [
		setProjectPath,
		setSessions,
		setActiveSessionId,
		setCapabilities,
		setActiveAgentId,
		setAgentConnected,
		setReplayingHistory,
	])

	async function handleOpenProject(path: string, options?: OpenProjectOptions) {
		const rememberOutgoing = options?.rememberOutgoing ?? true
		setLoading(true)
		try {
			if (
				rememberOutgoing &&
				projectPath &&
				!isGeneralChatProject(projectPath) &&
				projectPath !== path
			) {
				addRecentProject(projectPath)
			}
			if (!isGeneralChatProject(path)) {
				addRecentProject(path)
				setLastProjectPath(path)
			} else {
				setLastProjectPath(null)
			}
			setMessages([])
			setStreamingText("")
			setConfigOptions([])
			setActiveSessionId(null)
			const status = await openProject(path, {
				agentId: getAppSettings().defaultProvider,
				deferSessionBootstrap: true,
			})
			applyProjectStatus(status, {
				setAgentConnected,
				setProjectPath,
				setSessions,
				setActiveSessionId,
				setCapabilities,
				setActiveAgentId,
			}, { skipActiveSession: true })
		} finally {
			setLoading(false)
		}
	}

	async function handleCloseProject() {
		setLoading(true)
		try {
			const status = await closeProject()
			setLastProjectPath(null)
			setAgentConnected(status.connected)
			setProjectPath(status.projectPath)
			setSessions(status.sessions)
			setActiveSessionId(null)
			setCapabilities(null)
			setMessages([])
			setStreamingText("")
			setConfigOptions([])
		} finally {
			setLoading(false)
		}
	}

	return (
		<SidebarLayout
			appBar={settingsOpen ? <SettingsTitle /> : <SessionTitle />}
			sidebar={
				settingsOpen ? (
					<SettingsSidebar />
				) : (
					<AppSidebar
						connected={agentConnected}
						projectPath={projectPath}
						sessionStatus={sessionStatus}
						onOpenProject={handleOpenProject}
						onCloseProject={handleCloseProject}
						loading={loading}
					/>
				)
			}
		>
			{settingsOpen ? (
				<SettingsView />
			) : (
				<ChatView connected={agentConnected} onOpenProject={handleOpenProject} />
			)}
		</SidebarLayout>
	)
}