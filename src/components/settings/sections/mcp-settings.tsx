import { useAtomValue } from "jotai"
import { useCallback, useEffect, useMemo, useState } from "react"
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

const SOURCE_LABELS: Record<string, string> = {
	opencode: "OpenCode",
	cursor: "Cursor",
	claude: "Claude Desktop",
	minimax: "MiniMax",
}

const SCOPE_LABELS: Record<string, string> = {
	global: "Global",
	project: "Proyecto",
	managed: "Sistema",
}

function serverKey(server: OpencodeMcpServerEntry): string {
	return `${server.source}:${server.scope}:${server.name}`
}

function groupLabel(source: string, scope: string): string {
	const sourceLabel = SOURCE_LABELS[source] ?? source
	const scopeLabel = SCOPE_LABELS[scope] ?? scope
	return `${sourceLabel} · ${scopeLabel}`
}

function shortenPath(path: string): string {
	const match = /^\/Users\/[^/]+(\/.*)?$/.exec(path)
	if (match) {
		return match[1] ? `~${match[1]}` : "~"
	}
	return path
}

function formatDescription(server: OpencodeMcpServerEntry): string {
	const parts = [
		server.serverType,
		shortenPath(server.configPath),
		server.readOnly ? "solo lectura" : null,
	].filter(Boolean)
	return parts.join(" · ")
}

function McpServerList({
	servers,
	pendingKey,
	onToggle,
}: {
	servers: OpencodeMcpServerEntry[]
	pendingKey: string | null
	onToggle: (server: OpencodeMcpServerEntry, enabled: boolean) => void
}) {
	return (
		<SettingsGroup>
			{servers.map((server) => {
				const key = serverKey(server)
				return (
					<SettingsRow key={key} label={server.name} description={formatDescription(server)}>
						{server.readOnly ? (
							<span
								className="rounded border border-border px-1.5 py-0.5 text-[10px] uppercase text-muted-foreground"
								title="Detectado desde otra app; edítalo en su config original"
							>
								{server.enabled ? "activo" : "inactivo"}
							</span>
						) : (
							<SettingsToggle
								checked={server.enabled}
								disabled={pendingKey === key}
								ariaLabel={`${server.enabled ? "Desactivar" : "Activar"} ${server.name}`}
								onChange={(checked) => void onToggle(server, checked)}
							/>
						)}
					</SettingsRow>
				)
			})}
		</SettingsGroup>
	)
}

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

	const groupedServers = useMemo(() => {
		const groups = new Map<string, OpencodeMcpServerEntry[]>()
		for (const server of servers) {
			const label = groupLabel(server.source, server.scope)
			const current = groups.get(label) ?? []
			current.push(server)
			groups.set(label, current)
		}

		return [...groups.entries()]
			.map(([label, entries]) => ({
				label,
				entries: entries.sort((a, b) => a.name.localeCompare(b.name)),
			}))
			.sort((a, b) => a.label.localeCompare(b.label))
	}, [servers])

	const editableCount = servers.filter((server) => !server.readOnly).length
	const detectedCount = servers.length

	async function handleToggle(server: OpencodeMcpServerEntry, enabled: boolean) {
		const key = serverKey(server)
		setPendingKey(key)
		try {
			await setOpencodeMcpEnabled({
				name: server.name,
				scope: server.scope,
				enabled,
				projectPath: server.scope === "project" ? projectPath : null,
				configPath: server.configPath,
			})
			setServers((current) =>
				current.map((entry) => (serverKey(entry) === key ? { ...entry, enabled } : entry)),
			)
			window.dispatchEvent(new CustomEvent("circulo:mcp-changed"))
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
					description="Detecta MCP en OpenCode, Cursor, Claude y otras configs habituales. Los de OpenCode se pueden activar o desactivar aquí."
				/>
				{loading ? (
					<SettingsEmptyState>Cargando servidores MCP…</SettingsEmptyState>
				) : error ? (
					<SettingsEmptyState>{error}</SettingsEmptyState>
				) : servers.length === 0 ? (
					<SettingsEmptyState>
						No se encontraron servidores MCP en las rutas conocidas.
					</SettingsEmptyState>
				) : (
					<div className="space-y-5">
						<p className="text-xs text-muted-foreground">
							{detectedCount} detectados · {editableCount} editables en OpenCode
						</p>
						{groupedServers.map((group) => (
							<div key={group.label}>
								<p className="mb-2 text-xs font-medium text-muted-foreground">{group.label}</p>
								<McpServerList
									servers={group.entries}
									pendingKey={pendingKey}
									onToggle={handleToggle}
								/>
							</div>
						))}
					</div>
				)}
			</div>

			<p className="text-xs text-muted-foreground">
				Rutas revisadas: <code className="font-mono">~/.config/opencode</code>, proyecto{" "}
				<code className="font-mono">opencode.json(c)</code>,{" "}
				<code className="font-mono">~/.cursor/mcp.json</code>, Claude Desktop y MiniMax.
				Reabre el proyecto para aplicar cambios en la sesión activa.
			</p>
		</div>
	)
}