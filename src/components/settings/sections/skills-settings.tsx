import { open } from "@tauri-apps/plugin-dialog"
import { ExternalLink, FolderOpen, Search, Sparkles, X } from "lucide-react"
import { useAtomValue } from "jotai"
import { useCallback, useEffect, useMemo, useState } from "react"
import {
	SettingsBadge,
	SettingsCollapsible,
	SettingsEmptyState,
	SettingsGroup,
	SettingsRow,
	SettingsSectionHeader,
	SettingsSelect,
	SettingsTabs,
} from "@/components/settings/settings-ui"
import { formatMcpDisplayName } from "@/lib/mcp-display"
import { getProjectDisplayName, isGeneralChatProject } from "@/lib/project-display"
import {
	addRecentProject,
	getActiveProjectPaths,
	getRecentProjectLabel,
} from "@/lib/recent-projects"
import {
	buildSkillsShPackage,
	buildSkillsShUrl,
	formatSkillsShInstalls,
	searchSkillsSh,
	type SkillsShSearchResult,
} from "@/lib/skills-sh"
import {
	installSkillsShSkill,
	listOpencodeSkills,
	type OpencodeSkillEntry,
} from "@/lib/tauri"
import { cn } from "@/lib/utils"
import { projectPathAtom } from "@/stores/atoms"

type SkillsTab = "installed" | "explore"
type InstallScope = "global" | "project"

function shortenPath(path: string): string {
	const match = /^\/Users\/[^/]+(\/.*)?$/.exec(path)
	if (match) {
		return match[1] ? `~${match[1]}` : "~"
	}
	return path
}

function InstalledSkillsList({
	skills,
	emptyLabel,
}: {
	skills: OpencodeSkillEntry[]
	emptyLabel: string
}) {
	if (skills.length === 0) {
		return <SettingsEmptyState>{emptyLabel}</SettingsEmptyState>
	}

	return (
		<SettingsGroup>
			{skills.map((skill) => (
				<SettingsRow
					key={`${skill.scope}-${skill.path}`}
					label={formatMcpDisplayName(skill.name)}
					description={skill.description ?? shortenPath(skill.path)}
				>
					<SettingsBadge tone="muted">{skill.scope}</SettingsBadge>
				</SettingsRow>
			))}
		</SettingsGroup>
	)
}

function ExploreSkillRow({
	result,
	scope,
	projectPath,
	isInstalled,
	onInstalled,
}: {
	result: SkillsShSearchResult
	scope: InstallScope
	projectPath: string | null
	isInstalled: boolean
	onInstalled: () => void
}) {
	const [error, setError] = useState<string | null>(null)
	const [isInstalling, setIsInstalling] = useState(false)

	async function handleInstall() {
		if (scope === "project" && !projectPath) {
			setError("Selecciona un proyecto destino")
			return
		}

		setError(null)
		setIsInstalling(true)
		try {
			await installSkillsShSkill({
				package: buildSkillsShPackage(result),
				scope,
				projectPath: scope === "project" ? projectPath : null,
			})
			onInstalled()
		} catch (err) {
			setError(err instanceof Error ? err.message : "No se pudo instalar el skill")
		} finally {
			setIsInstalling(false)
		}
	}

	return (
		<SettingsRow
			label={formatMcpDisplayName(result.name)}
			description={`${result.source} · ${formatSkillsShInstalls(result.installs)} installs`}
			className="py-2.5"
		>
			<div className="flex flex-col items-end gap-1">
				<div className="flex items-center gap-2">
					<a
						href={buildSkillsShUrl(result)}
						target="_blank"
						rel="noreferrer"
						className="rounded p-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
						aria-label={`Abrir ${result.name} en skills.sh`}
					>
						<ExternalLink className="size-3.5" />
					</a>
					<button
						type="button"
						disabled={isInstalled || isInstalling}
						onClick={() => void handleInstall()}
						className={cn(
							"h-7 rounded-md px-2.5 text-xs font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50",
							isInstalled
								? "bg-muted text-muted-foreground"
								: "bg-primary text-primary-foreground hover:bg-primary/90",
						)}
					>
						{isInstalling ? "Instalando…" : isInstalled ? "Instalado" : "Instalar"}
					</button>
				</div>
				{error ? <p className="max-w-[12rem] text-right text-[10px] text-destructive">{error}</p> : null}
			</div>
		</SettingsRow>
	)
}

function defaultSkillsProjectPath(projectPath: string | null, savedProjects: string[]): string | null {
	if (projectPath && !isGeneralChatProject(projectPath)) return projectPath
	return savedProjects[0] ?? null
}

export function SkillsSettings() {
	const projectPath = useAtomValue(projectPathAtom)
	const [tab, setTab] = useState<SkillsTab>("installed")
	const [skills, setSkills] = useState<OpencodeSkillEntry[]>([])
	const [loading, setLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)
	const [recentProjectsVersion, setRecentProjectsVersion] = useState(0)

	const savedProjects = useMemo(() => {
		void recentProjectsVersion
		return getActiveProjectPaths(projectPath)
	}, [projectPath, recentProjectsVersion])

	const [skillsProjectPath, setSkillsProjectPath] = useState<string | null>(() =>
		defaultSkillsProjectPath(projectPath, getActiveProjectPaths(projectPath)),
	)

	const [query, setQuery] = useState("")
	const [debouncedQuery, setDebouncedQuery] = useState("")
	const [searchResults, setSearchResults] = useState<SkillsShSearchResult[]>([])
	const [searchLoading, setSearchLoading] = useState(false)
	const [searchError, setSearchError] = useState<string | null>(null)
	const [installScope, setInstallScope] = useState<InstallScope>("global")
	const [expandedInstalled, setExpandedInstalled] = useState<Record<string, boolean>>({
		global: false,
		project: false,
	})

	useEffect(() => {
		if (skillsProjectPath && savedProjects.includes(skillsProjectPath)) return
		setSkillsProjectPath(defaultSkillsProjectPath(projectPath, savedProjects))
	}, [projectPath, savedProjects, skillsProjectPath])

	const refresh = useCallback(async () => {
		setLoading(true)
		setError(null)
		try {
			const entries = await listOpencodeSkills(skillsProjectPath)
			setSkills(entries)
		} catch (err) {
			setError(err instanceof Error ? err.message : "No se pudieron cargar los skills")
		} finally {
			setLoading(false)
		}
	}, [skillsProjectPath])

	useEffect(() => {
		void refresh()
	}, [refresh])

	useEffect(() => {
		const timer = window.setTimeout(() => setDebouncedQuery(query.trim()), 300)
		return () => window.clearTimeout(timer)
	}, [query])

	useEffect(() => {
		if (tab !== "explore" || !debouncedQuery) {
			setSearchResults([])
			setSearchError(null)
			setSearchLoading(false)
			return
		}

		let cancelled = false
		setSearchLoading(true)
		setSearchError(null)

		void searchSkillsSh(debouncedQuery)
			.then((results) => {
				if (!cancelled) setSearchResults(results)
			})
			.catch((err) => {
				if (!cancelled) {
					setSearchError(err instanceof Error ? err.message : "Error al buscar en skills.sh")
					setSearchResults([])
				}
			})
			.finally(() => {
				if (!cancelled) setSearchLoading(false)
			})

		return () => {
			cancelled = true
		}
	}, [tab, debouncedQuery])

	const installedNames = useMemo(() => {
		const scoped =
			installScope === "global"
				? skills.filter((skill) => skill.scope === "global")
				: skills.filter((skill) => skill.scope === "project")
		return new Set(scoped.map((skill) => skill.name.toLowerCase()))
	}, [skills, installScope])

	async function handleBrowseProject() {
		const selected = await open({
			directory: true,
			multiple: false,
			title: "Seleccionar proyecto",
		})
		if (!selected || Array.isArray(selected)) return
		addRecentProject(selected)
		setRecentProjectsVersion((value) => value + 1)
		setSkillsProjectPath(selected)
	}

	const globalSkills = skills.filter((skill) => skill.scope === "global")
	const projectSkills = skills.filter((skill) => skill.scope === "project")

	async function handleInstalledFromExplore() {
		await refresh()
		setTab("installed")
	}

	return (
		<div className="space-y-5">
			<div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
				<SettingsSectionHeader
					title="Skills"
					description="Gestiona skills instalados o explora el catálogo de skills.sh para añadir nuevos."
				/>
				<SettingsTabs
					tabs={[
						{ id: "installed", label: "Instalados" },
						{ id: "explore", label: "Explorar" },
					]}
					active={tab}
					onChange={setTab}
				/>
			</div>

			{tab === "installed" ? (
				<>
					<div className="flex flex-wrap items-center gap-2">
						<SettingsBadge tone="neutral">{skills.length} skills</SettingsBadge>
						<SettingsBadge tone="muted">{globalSkills.length} global</SettingsBadge>
						<SettingsBadge tone="muted">{projectSkills.length} proyecto</SettingsBadge>
					</div>

					{loading ? (
						<SettingsEmptyState>Cargando skills…</SettingsEmptyState>
					) : error ? (
						<SettingsEmptyState>{error}</SettingsEmptyState>
					) : (
						<div className="space-y-3">
							<SettingsCollapsible
								title="Global"
								subtitle="~/.config/opencode/skills"
								icon={<Sparkles className="size-4" />}
								open={expandedInstalled.global}
								onOpenChange={(open) =>
									setExpandedInstalled((current) => ({ ...current, global: open }))
								}
								badges={<SettingsBadge tone="neutral">{globalSkills.length}</SettingsBadge>}
							>
								<InstalledSkillsList
									skills={globalSkills}
									emptyLabel="No hay skills globales instalados."
								/>
							</SettingsCollapsible>

							<SettingsCollapsible
								title={`Proyecto — ${getProjectDisplayName(skillsProjectPath)}`}
								subtitle=".agents/skills y .opencode/skills"
								icon={<Sparkles className="size-4" />}
								open={expandedInstalled.project}
								onOpenChange={(open) =>
									setExpandedInstalled((current) => ({ ...current, project: open }))
								}
								badges={<SettingsBadge tone="neutral">{projectSkills.length}</SettingsBadge>}
							>
								{savedProjects.length === 0 && !skillsProjectPath ? (
									<div className="space-y-3 px-4 pb-3">
										<SettingsEmptyState>
											Añade un proyecto desde el sidebar o elige una carpeta.
										</SettingsEmptyState>
										<div className="flex justify-end">
											<button
												type="button"
												onClick={() => void handleBrowseProject()}
												className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border/60 px-2.5 text-xs text-foreground transition-colors hover:bg-muted"
											>
												<FolderOpen className="size-3.5 text-muted-foreground" />
												Carpeta…
											</button>
										</div>
									</div>
								) : (
									<div className="space-y-3 px-4 pb-3">
										<div className="flex flex-wrap items-center justify-end gap-2">
											<SettingsSelect
												value={skillsProjectPath ?? ""}
												onChange={setSkillsProjectPath}
												options={savedProjects.map((path) => ({
													value: path,
													label: getRecentProjectLabel(path),
												}))}
											/>
											<button
												type="button"
												onClick={() => void handleBrowseProject()}
												className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border/60 px-2.5 text-xs text-foreground transition-colors hover:bg-muted"
											>
												<FolderOpen className="size-3.5 text-muted-foreground" />
												Carpeta…
											</button>
										</div>
										{!skillsProjectPath ? (
											<SettingsEmptyState>
												Selecciona un proyecto para ver sus skills locales.
											</SettingsEmptyState>
										) : (
											<InstalledSkillsList
												skills={projectSkills}
												emptyLabel="No hay skills en este proyecto."
											/>
										)}
									</div>
								)}
							</SettingsCollapsible>
						</div>
					)}
				</>
			) : (
				<>
					<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
						<label className="relative min-w-0 flex-1">
							<Search className="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
							<input
								type="search"
								value={query}
								onChange={(event) => setQuery(event.target.value)}
								placeholder="Buscar en skills.sh — react, tauri, design…"
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

						<div className="flex shrink-0 items-center gap-2">
							<span className="text-xs text-muted-foreground">Instalar en</span>
							<SettingsTabs
								tabs={[
									{ id: "global", label: "Global" },
									{ id: "project", label: "Proyecto" },
								]}
								active={installScope}
								onChange={setInstallScope}
							/>
						</div>
					</div>

					{installScope === "project" ? (
						<div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border/60 bg-muted/20 px-3 py-2">
							<span className="text-xs text-muted-foreground">Proyecto destino</span>
							<div className="flex flex-wrap items-center gap-2">
								{savedProjects.length > 0 ? (
									<SettingsSelect
										value={skillsProjectPath ?? ""}
										onChange={setSkillsProjectPath}
										options={savedProjects.map((path) => ({
											value: path,
											label: getRecentProjectLabel(path),
										}))}
									/>
								) : (
									<span className="text-xs text-amber-600 dark:text-amber-400">
										Añade un proyecto o elige una carpeta.
									</span>
								)}
								<button
									type="button"
									onClick={() => void handleBrowseProject()}
									className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border/60 px-2.5 text-xs text-foreground transition-colors hover:bg-muted"
								>
									<FolderOpen className="size-3.5 text-muted-foreground" />
									Carpeta…
								</button>
							</div>
						</div>
					) : null}

					{!debouncedQuery ? (
						<SettingsEmptyState>
							Escribe para buscar skills en{" "}
							<a
								href="https://skills.sh"
								target="_blank"
								rel="noreferrer"
								className="text-foreground underline underline-offset-2"
							>
								skills.sh
							</a>
							.
						</SettingsEmptyState>
					) : searchLoading ? (
						<SettingsEmptyState>Buscando en skills.sh…</SettingsEmptyState>
					) : searchError ? (
						<SettingsEmptyState>{searchError}</SettingsEmptyState>
					) : searchResults.length === 0 ? (
						<SettingsEmptyState>
							Sin resultados para &ldquo;{debouncedQuery}&rdquo;.
						</SettingsEmptyState>
					) : (
						<SettingsGroup>
							{searchResults.map((result) => {
								const isInstalled = installedNames.has(result.skillId.toLowerCase())
								return (
									<ExploreSkillRow
										key={result.id}
										result={result}
										scope={installScope}
										projectPath={skillsProjectPath}
										isInstalled={isInstalled}
										onInstalled={() => void handleInstalledFromExplore()}
									/>
								)
							})}
						</SettingsGroup>
					)}
				</>
			)}

			<p className="text-xs leading-relaxed text-muted-foreground">
				La instalación usa <code className="font-mono">npx skills add</code> con agente OpenCode.
				Global va a <code className="font-mono">~/.config/opencode/skills</code>; proyecto a{" "}
				<code className="font-mono">.agents/skills</code> del repo que elijas — no hace falta tenerlo
				abierto en el chat.
			</p>
		</div>
	)
}