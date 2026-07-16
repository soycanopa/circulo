import {
	Bot,
	ChevronDown,
	ChevronUp,
	Monitor,
	Plug,
	Search,
	Sparkles,
	X,
} from "lucide-react"
import type { LucideIcon } from "lucide-react"
import { useAtomValue } from "jotai"
import { useCallback, useEffect, useMemo, useState } from "react"
import {
	SettingsBadge,
	SettingsCollapsible,
	SettingsEmptyState,
	SettingsRow,
	SettingsSectionHeader,
	SettingsToggle,
} from "@/components/settings/settings-ui"
import { formatMcpDisplayName } from "@/lib/mcp-display"
import {
	listOpencodeMcpServers,
	setOpencodeMcpEnabled,
	type OpencodeMcpServerEntry,
} from "@/lib/tauri"
import { cn } from "@/lib/utils"
import { projectPathAtom } from "@/stores/atoms"

const SOURCE_META: Record<string, { label: string; icon: LucideIcon }> = {
	opencode: { label: "OpenCode", icon: Plug },
	cursor: { label: "Cursor", icon: Monitor },
	claude: { label: "Claude", icon: Sparkles },
	minimax: { label: "MiniMax", icon: Bot },
}

const SCOPE_LABELS: Record<string, string> = {
	global: "Global",
	project: "Proyecto",
	managed: "Sistema",
}

const SOURCE_ORDER = ["opencode", "cursor", "claude", "minimax"]

type ScopeGroup = {
	scope: string
	label: string
	configHint: string
	servers: OpencodeMcpServerEntry[]
	activeCount: number
}

type AgentGroup = {
	source: string
	label: string
	icon: LucideIcon
	readOnly: boolean
	scopes: ScopeGroup[]
	totalCount: number
	activeCount: number
}

function serverKey(server: OpencodeMcpServerEntry): string {
	return `${server.source}:${server.scope}:${server.name}`
}

function shortenPath(path: string): string {
	const match = /^\/Users\/[^/]+(\/.*)?$/.exec(path)
	if (match) {
		return match[1] ? `~${match[1]}` : "~"
	}
	return path
}

function sourceMeta(source: string) {
	return (
		SOURCE_META[source] ?? {
			label: source.charAt(0).toUpperCase() + source.slice(1),
			icon: Plug,
		}
	)
}

function buildAgentGroups(servers: OpencodeMcpServerEntry[]): AgentGroup[] {
	const bySource = new Map<string, Map<string, OpencodeMcpServerEntry[]>>()

	for (const server of servers) {
		const scopes = bySource.get(server.source) ?? new Map()
		const list = scopes.get(server.scope) ?? []
		list.push(server)
		scopes.set(server.scope, list)
		bySource.set(server.source, scopes)
	}

	const groups: AgentGroup[] = []

	for (const [source, scopesMap] of bySource) {
		const meta = sourceMeta(source)
		const scopes: ScopeGroup[] = [...scopesMap.entries()]
			.map(([scope, entries]) => {
				const sorted = [...entries].sort((a, b) => a.name.localeCompare(b.name))
				return {
					scope,
					label: SCOPE_LABELS[scope] ?? scope,
					configHint: shortenPath(sorted[0]?.configPath ?? ""),
					servers: sorted,
					activeCount: sorted.filter((entry) => entry.enabled).length,
				}
			})
			.sort((a, b) => a.label.localeCompare(b.label))

		const allServers = scopes.flatMap((scope) => scope.servers)
		groups.push({
			source,
			label: meta.label,
			icon: meta.icon,
			readOnly: allServers.every((entry) => entry.readOnly),
			scopes,
			totalCount: allServers.length,
			activeCount: allServers.filter((entry) => entry.enabled).length,
		})
	}

	return groups.sort((a, b) => {
		const aIndex = SOURCE_ORDER.indexOf(a.source)
		const bIndex = SOURCE_ORDER.indexOf(b.source)
		const aRank = aIndex === -1 ? Number.MAX_SAFE_INTEGER : aIndex
		const bRank = bIndex === -1 ? Number.MAX_SAFE_INTEGER : bIndex
		if (aRank !== bRank) return aRank - bRank
		return a.label.localeCompare(b.label)
	})
}

function matchesQuery(server: OpencodeMcpServerEntry, query: string): boolean {
	const haystack = [
		server.name,
		formatMcpDisplayName(server.name),
		server.source,
		server.scope,
		server.serverType,
		server.configPath,
	]
		.filter(Boolean)
		.join(" ")
		.toLowerCase()
	return haystack.includes(query)
}

function McpServerRow({
	server,
	pendingKey,
	onToggle,
}: {
	server: OpencodeMcpServerEntry
	pendingKey: string | null
	onToggle: (server: OpencodeMcpServerEntry, enabled: boolean) => void
}) {
	const key = serverKey(server)
	const displayName = formatMcpDisplayName(server.name)

	return (
		<SettingsRow
			label={displayName}
			description={shortenPath(server.configPath)}
			className="py-2.5"
		>
			<div className="flex items-center gap-2">
				{server.serverType ? (
					<SettingsBadge tone="muted">{server.serverType}</SettingsBadge>
				) : null}
				{server.readOnly ? (
					<span
						className={cn(
							"inline-flex items-center gap-1.5 rounded-md px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide",
							server.enabled
								? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
								: "bg-muted text-muted-foreground",
						)}
						title="Detectado desde otra app; edítalo en su config original"
					>
						<span
							className={cn(
								"size-1.5 rounded-full",
								server.enabled ? "bg-emerald-500" : "bg-muted-foreground/50",
							)}
						/>
						{server.enabled ? "activo" : "inactivo"}
					</span>
				) : (
					<SettingsToggle
						checked={server.enabled}
						disabled={pendingKey === key}
						ariaLabel={`${server.enabled ? "Desactivar" : "Activar"} ${displayName}`}
						onChange={(checked) => void onToggle(server, checked)}
					/>
				)}
			</div>
		</SettingsRow>
	)
}

export function McpSettings() {
	const projectPath = useAtomValue(projectPathAtom)
	const [servers, setServers] = useState<OpencodeMcpServerEntry[]>([])
	const [loading, setLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)
	const [pendingKey, setPendingKey] = useState<string | null>(null)
	const [query, setQuery] = useState("")
	const [expandedAgents, setExpandedAgents] = useState<Record<string, boolean>>({})
	const [expandedScopes, setExpandedScopes] = useState<Record<string, boolean>>({})

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

	const normalizedQuery = query.trim().toLowerCase()

	const filteredServers = useMemo(() => {
		if (!normalizedQuery) return servers
		return servers.filter((server) => matchesQuery(server, normalizedQuery))
	}, [servers, normalizedQuery])

	const agentGroups = useMemo(() => buildAgentGroups(filteredServers), [filteredServers])

	useEffect(() => {
		if (servers.length === 0) return
		setExpandedAgents((current) => {
			if (Object.keys(current).length > 0) return current
			return Object.fromEntries(
				buildAgentGroups(servers).map((group) => [group.source, false]),
			)
		})
		setExpandedScopes((current) => {
			if (Object.keys(current).length > 0) return current
			const initial: Record<string, boolean> = {}
			for (const group of buildAgentGroups(servers)) {
				for (const scope of group.scopes) {
					initial[`${group.source}:${scope.scope}`] = false
				}
			}
			return initial
		})
	}, [servers])

	useEffect(() => {
		if (!normalizedQuery) return
		const nextAgents: Record<string, boolean> = {}
		const nextScopes: Record<string, boolean> = {}
		for (const group of agentGroups) {
			nextAgents[group.source] = true
			for (const scope of group.scopes) {
				nextScopes[`${group.source}:${scope.scope}`] = true
			}
		}
		setExpandedAgents((current) => ({ ...current, ...nextAgents }))
		setExpandedScopes((current) => ({ ...current, ...nextScopes }))
	}, [normalizedQuery, agentGroups])

	const stats = useMemo(() => {
		const active = servers.filter((server) => server.enabled).length
		const editable = servers.filter((server) => !server.readOnly).length
		const agents = new Set(servers.map((server) => server.source)).size
		return { total: servers.length, active, editable, agents }
	}, [servers])

	function setAllExpanded(expanded: boolean) {
		const agents: Record<string, boolean> = {}
		const scopes: Record<string, boolean> = {}
		for (const group of agentGroups) {
			agents[group.source] = expanded
			for (const scope of group.scopes) {
				scopes[`${group.source}:${scope.scope}`] = expanded
			}
		}
		setExpandedAgents(agents)
		setExpandedScopes(scopes)
	}

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
		<div className="space-y-5">
			<SettingsSectionHeader
				title="Servidores MCP"
				description="Agrupados por agente y alcance. A medida que Circulo detecte más configs, cada fuente aparece en su propio bloque desplegable."
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
				<>
					<div className="flex flex-wrap items-center gap-2">
						<SettingsBadge tone="neutral">{stats.total} servidores</SettingsBadge>
						<SettingsBadge tone="success">{stats.active} activos</SettingsBadge>
						<SettingsBadge tone="accent">{stats.editable} editables</SettingsBadge>
						<SettingsBadge tone="muted">
							{stats.agents} {stats.agents === 1 ? "agente" : "agentes"}
						</SettingsBadge>
					</div>

					<div className="flex flex-col gap-2 sm:flex-row sm:items-center">
						<label className="relative min-w-0 flex-1">
							<Search className="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
							<input
								type="search"
								value={query}
								onChange={(event) => setQuery(event.target.value)}
								placeholder="Buscar por nombre, tipo o ruta…"
								className="h-9 w-full rounded-lg border border-border/60 bg-background pl-9 pr-9 text-sm text-foreground outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring"
							/>
							{query ? (
								<button
									type="button"
									onClick={() => setQuery("")}
									className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
									aria-label="Limpiar búsqueda"
								>
									<X className="size-3.5" />
								</button>
							) : null}
						</label>
						<div className="flex shrink-0 items-center gap-1">
							<button
								type="button"
								onClick={() => setAllExpanded(true)}
								className="inline-flex items-center gap-1 rounded-md px-2 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
							>
								<ChevronDown className="size-3.5" />
								Expandir todo
							</button>
							<button
								type="button"
								onClick={() => setAllExpanded(false)}
								className="inline-flex items-center gap-1 rounded-md px-2 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
							>
								<ChevronUp className="size-3.5" />
								Colapsar todo
							</button>
						</div>
					</div>

					{filteredServers.length === 0 ? (
						<SettingsEmptyState>
							Ningún servidor coincide con &ldquo;{query}&rdquo;.
						</SettingsEmptyState>
					) : (
						<div className="space-y-3">
							{agentGroups.map((group) => {
								const Icon = group.icon
								return (
									<SettingsCollapsible
										key={group.source}
										title={group.label}
										subtitle={
											group.readOnly
												? "Solo lectura — edita en la app de origen"
												: "Editable desde Circulo"
										}
										icon={<Icon className="size-4" />}
										open={expandedAgents[group.source] ?? false}
										onOpenChange={(open) =>
											setExpandedAgents((current) => ({ ...current, [group.source]: open }))
										}
										badges={
											<>
												<SettingsBadge tone="neutral">{group.totalCount}</SettingsBadge>
												<SettingsBadge tone="success">{group.activeCount} on</SettingsBadge>
											</>
										}
									>
										<div className="space-y-2">
											{group.scopes.map((scope) => {
												const scopeKey = `${group.source}:${scope.scope}`
												const singleScope = group.scopes.length === 1

												if (singleScope) {
													return (
														<div
															key={scopeKey}
															className="divide-y divide-border/50 rounded-lg border border-border/40 bg-background/50"
														>
															{scope.servers.map((server) => (
																<McpServerRow
																	key={serverKey(server)}
																	server={server}
																	pendingKey={pendingKey}
																	onToggle={handleToggle}
																/>
															))}
														</div>
													)
												}

												return (
													<SettingsCollapsible
														key={scopeKey}
														level="nested"
														title={scope.label}
														subtitle={scope.configHint}
														open={expandedScopes[scopeKey] ?? false}
														onOpenChange={(open) =>
															setExpandedScopes((current) => ({
																...current,
																[scopeKey]: open,
															}))
														}
														badges={
															<>
																<SettingsBadge tone="neutral">{scope.servers.length}</SettingsBadge>
																<SettingsBadge tone="success">{scope.activeCount} on</SettingsBadge>
															</>
														}
													>
														<div className="divide-y divide-border/50 rounded-md border border-border/30 bg-background/60">
															{scope.servers.map((server) => (
																<McpServerRow
																	key={serverKey(server)}
																	server={server}
																	pendingKey={pendingKey}
																	onToggle={handleToggle}
																/>
															))}
														</div>
													</SettingsCollapsible>
												)
											})}
										</div>
									</SettingsCollapsible>
								)
							})}
						</div>
					)}
				</>
			)}

			<p className="text-xs leading-relaxed text-muted-foreground">
				Fuentes: OpenCode, Cursor, Claude Desktop, MiniMax y más según se agreguen agentes.
				Reabre el proyecto para aplicar cambios en la sesión activa.
			</p>
		</div>
	)
}