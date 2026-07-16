import { useAtom, useSetAtom } from "jotai"
import { useEffect, useState } from "react"
import { SessionTitle } from "@/components/chat/session-title"
import { AppSidebar } from "@/components/layout/app-sidebar"
import { SidebarLayout } from "@/components/layout/sidebar-layout"
import { ChatView } from "@/components/chat/chat-view"
import { SettingsSidebar } from "@/components/settings/settings-sidebar"
import { SettingsTitle } from "@/components/settings/settings-title"
import { SettingsView } from "@/components/settings/settings-view"
import { addRecentProject } from "@/lib/recent-projects"
import { getChatsProjectPath } from "@/lib/app-settings"
import { isGeneralChatProject } from "@/lib/project-display"
import { closeProject, getProjectStatus, openProject } from "@/lib/tauri"
import {
	activeSessionIdAtom,
	agentCapabilitiesAtom,
	diffPanelOpenAtom,
	projectPathAtom,
	sessionStatusAtom,
	sessionsAtom,
	settingsOpenAtom,
	terminalOpenAtom,
} from "@/stores/atoms"

export function AppShell() {
	const [projectPath, setProjectPath] = useAtom(projectPathAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const setActiveSessionId = useSetAtom(activeSessionIdAtom)
	const setCapabilities = useSetAtom(agentCapabilitiesAtom)
	const [sessionStatus] = useAtom(sessionStatusAtom)
	const [settingsOpen, setSettingsOpen] = useAtom(settingsOpenAtom)
	const setDiffPanelOpen = useSetAtom(diffPanelOpenAtom)
	const setTerminalOpen = useSetAtom(terminalOpenAtom)
	const [connected, setConnected] = useState(false)
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
		setConnected(sessionStatus !== "disconnected")
	}, [sessionStatus])

	useEffect(() => {
		let cancelled = false

		async function bootstrap() {
			try {
				let status = await getProjectStatus()
				if (!status.connected) {
					status = await openProject(getChatsProjectPath())
				}
				if (
					status.connected &&
					status.sessions.length === 0 &&
					isGeneralChatProject(status.projectPath)
				) {
					status = await closeProject()
				}
				if (cancelled) return
				setConnected(status.connected)
				setProjectPath(status.projectPath)
				setSessions(status.sessions)
				setActiveSessionId(status.activeSessionId ?? status.sessionId)
				setCapabilities(status.capabilities)
			} catch {
				if (!cancelled) setConnected(false)
			}
		}

		void bootstrap()
		return () => {
			cancelled = true
		}
	}, [setProjectPath, setSessions, setActiveSessionId, setCapabilities])

	async function handleOpenProject(path: string) {
		setLoading(true)
		try {
			addRecentProject(path)
			const status = await openProject(path)
			setConnected(status.connected)
			setProjectPath(status.projectPath)
			setSessions(status.sessions)
			setActiveSessionId(status.activeSessionId ?? status.sessionId)
			setCapabilities(status.capabilities)
		} finally {
			setLoading(false)
		}
	}

	async function handleCloseProject() {
		setLoading(true)
		try {
			const status = await closeProject()
			setConnected(status.connected)
			setProjectPath(status.projectPath)
			setSessions(status.sessions)
			setActiveSessionId(null)
			setCapabilities(null)
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
						connected={connected}
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
				<ChatView connected={connected} onOpenProject={handleOpenProject} />
			)}
		</SidebarLayout>
	)
}
