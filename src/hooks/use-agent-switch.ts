import { useSetAtom } from "jotai"
import { useCallback } from "react"
import { persistAppSettings } from "@/lib/app-settings"
import type { AgentProviderId } from "@/lib/agent-providers"
import { getAgentProvider } from "@/lib/agent-providers"
import { openProject } from "@/lib/tauri"
import {
	activeAgentIdAtom,
	activeSessionIdAtom,
	agentCapabilitiesAtom,
	appSettingsAtom,
	errorMessageAtom,
	projectPathAtom,
	sessionsAtom,
} from "@/stores/atoms"

export function useAgentSwitch() {
	const setProjectPath = useSetAtom(projectPathAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const setActiveSessionId = useSetAtom(activeSessionIdAtom)
	const setCapabilities = useSetAtom(agentCapabilitiesAtom)
	const setActiveAgentId = useSetAtom(activeAgentIdAtom)
	const setErrorMessage = useSetAtom(errorMessageAtom)
	const setAppSettings = useSetAtom(appSettingsAtom)

	const switchAgent = useCallback(
		async (agentId: AgentProviderId, projectPath: string | null) => {
			const provider = getAgentProvider(agentId)
			if (!provider?.acpReady) {
				setErrorMessage(`${provider?.label ?? agentId} aún no está disponible en Circulo.`)
				return false
			}

			setAppSettings(persistAppSettings({ defaultProvider: agentId }))
			setActiveAgentId(agentId)

			if (!projectPath) return true

			try {
				const status = await openProject(projectPath, {
					agentId,
					deferSessionBootstrap: true,
				})
				setProjectPath(status.projectPath)
				setSessions(status.sessions)
				const sessionId = status.activeSessionId ?? status.sessionId
				if (sessionId && sessionId !== "pending") {
					setActiveSessionId(sessionId)
				}
				setCapabilities(status.capabilities)
				if (status.agentId) setActiveAgentId(status.agentId as AgentProviderId)
				setErrorMessage(null)
				return true
			} catch (err) {
				const message = err instanceof Error ? err.message : String(err)
				setErrorMessage(message)
				return false
			}
		},
		[
			setActiveAgentId,
			setActiveSessionId,
			setAppSettings,
			setCapabilities,
			setErrorMessage,
			setProjectPath,
			setSessions,
		],
	)

	return { switchAgent }
}