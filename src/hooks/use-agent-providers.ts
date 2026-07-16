import { useEffect, useMemo, useState } from "react"
import {
	AGENT_PROVIDERS,
	getAgentProvider,
	type AgentProviderDefinition,
	type AgentProviderId,
} from "@/lib/agent-providers"
import { getAppSettings } from "@/lib/app-settings"
import { listAgentProviderVersions, type AgentProviderVersion } from "@/lib/tauri"

export interface AgentProviderEntry extends AgentProviderDefinition {
	installed: boolean
	version: string | null
	installError: string | null
	selectable: boolean
}

export function useAgentProviders() {
	const [versions, setVersions] = useState<AgentProviderVersion[]>([])
	const [loading, setLoading] = useState(true)

	useEffect(() => {
		let cancelled = false

		async function load() {
			try {
				const result = await listAgentProviderVersions()
				if (!cancelled) setVersions(result)
			} catch {
				if (!cancelled) setVersions([])
			} finally {
				if (!cancelled) setLoading(false)
			}
		}

		void load()
		return () => {
			cancelled = true
		}
	}, [])

	const entries = useMemo<AgentProviderEntry[]>(() => {
		const versionById = new Map(versions.map((entry) => [entry.id, entry]))

		return AGENT_PROVIDERS.map((provider) => {
			const probe = versionById.get(provider.id)
			const installed = probe?.installed ?? false
			return {
				...provider,
				installed,
				version: probe?.version ?? null,
				installError: probe?.error ?? null,
				selectable: provider.acpReady && installed,
			}
		})
	}, [versions])

	const defaultProviderId = getAppSettings().defaultProvider

	return {
		entries,
		loading,
		defaultProviderId,
		getEntry: (id: AgentProviderId) => entries.find((entry) => entry.id === id),
		getDefinition: getAgentProvider,
	}
}