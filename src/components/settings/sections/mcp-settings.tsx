import { useAtomValue } from "jotai"
import { useCallback, useEffect, useState } from "react"
import {
	SettingsEmptyState,
	SettingsGroup,
	SettingsRow,
	SettingsSectionHeader,
	SettingsToggle,
} from "@/components/settings/settings-ui"
import {
	listOpencodeMcpServers,
	setOpencodeMcpEnabled,
	type OpencodeMcpServerEntry,
} from "@/lib/tauri"
import { projectPathAtom } from "@/stores/atoms"

export function McpSettings() {
	const projectPath = useAtomValue(projectPathAtom)
	const [servers, setServers] = useState<OpencodeMcpServerEntry[]>([])
	const [loading, setLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)
	const [pendingKey, setPendingKey] = useState<string | null>(null)

	const refresh = useCallback(async () => {
		setLoading(true)
		setError(null)
		try {
			const entries = await listOpencodeMcpServers(projectPath)
			setServers(entries)
		} catch (err) {
			setError(err instanceof Error ? err.message : "No se pudieron cargar los MCP")
		} finally {
			setLoading(false)
		}
	}, [projectPath])

	useEffect(() => {
		void refresh()
	}, [refresh])

	async function handleToggle(server: OpencodeMcpServerEntry, enabled: boolean) {
		const key = `${server.scope}:${server.name}`
		setPendingKey(key)
		try {
			await setOpencodeMcpEnabled({
				name: server.name,
				scope: server.scope,
				enabled,
				projectPath: server.scope === "project" ? projectPath : null,
			})
			setServers((current) =>
				current.map((entry) =>
					entry.name === server.name && entry.scope === server.scope
						? { ...entry, enabled }
						: entry,
				),
			)
		} catch (err) {
			setError(err instanceof Error ? err.message : "No se pudo actualizar el MCP")
		} finally {
			setPendingKey(null)
		}
	}

	return (
		<div className="space-y-6">
			<div>
				<SettingsSectionHeader
					title="Servidores MCP"
					description="Lee y escribe opencode.json. Reabre el proyecto para aplicar en la sesión activa."
				/>
				{loading ? (
					<SettingsEmptyState>Cargando servidores MCP…</SettingsEmptyState>
				) : error ? (
					<SettingsEmptyState>{error}</SettingsEmptyState>
				) : servers.length === 0 ? (
					<SettingsEmptyState>
						No hay servidores MCP en la config global ni del proyecto.
					</SettingsEmptyState>
				) : (
					<SettingsGroup>
						{servers.map((server) => {
							const key = `${server.scope}:${server.name}`
							return (
								<SettingsRow
									key={key}
									label={server.name}
									description={[server.scope, server.serverType].filter(Boolean).join(" · ")}
								>
									<SettingsToggle
										checked={server.enabled}
										disabled={pendingKey === key}
										ariaLabel={`${server.enabled ? "Desactivar" : "Activar"} ${server.name}`}
										onChange={(checked) => void handleToggle(server, checked)}
									/>
								</SettingsRow>
							)
						})}
					</SettingsGroup>
				)}
			</div>
		</div>
	)
}