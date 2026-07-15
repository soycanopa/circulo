import { useAtom, useSetAtom } from "jotai"
import { useEffect, useState } from "react"
import { SessionTitle } from "@/components/chat/session-title"
import { AppSidebar } from "@/components/layout/app-sidebar"
import { SidebarLayout } from "@/components/layout/sidebar-layout"
import { ChatView } from "@/components/chat/chat-view"
import { closeProject, getProjectStatus, openProject } from "@/lib/tauri"
import {
	activeSessionIdAtom,
	agentCapabilitiesAtom,
	projectPathAtom,
	sessionStatusAtom,
	sessionsAtom,
} from "@/stores/atoms"

export function AppShell() {
	const [projectPath, setProjectPath] = useAtom(projectPathAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const setActiveSessionId = useSetAtom(activeSessionIdAtom)
	const setCapabilities = useSetAtom(agentCapabilitiesAtom)
	const [sessionStatus] = useAtom(sessionStatusAtom)
	const [connected, setConnected] = useState(false)
	const [loading, setLoading] = useState(false)

	useEffect(() => {
		setConnected(sessionStatus !== "disconnected")
	}, [sessionStatus])

	useEffect(() => {
		getProjectStatus()
			.then((status) => {
				setConnected(status.connected)
				setProjectPath(status.projectPath)
				setSessions(status.sessions)
				setActiveSessionId(status.activeSessionId ?? status.sessionId)
				setCapabilities(status.capabilities)
			})
			.catch(() => setConnected(false))
	}, [setProjectPath, setSessions, setActiveSessionId, setCapabilities])

	async function handleOpenProject(path: string) {
		setLoading(true)
		try {
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
			appBar={<SessionTitle />}
			sidebar={
				<AppSidebar
					connected={connected}
					projectPath={projectPath}
					sessionStatus={sessionStatus}
					onOpenProject={handleOpenProject}
					onCloseProject={handleCloseProject}
					loading={loading}
				/>
			}
		>
			<ChatView connected={connected} />
		</SidebarLayout>
	)
}
