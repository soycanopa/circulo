import { Badge } from "@circulo/ui/components/badge"
import { Button } from "@circulo/ui/components/button"
import { Dialog, DialogContent } from "@circulo/ui/components/dialog"
import { Input } from "@circulo/ui/components/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@circulo/ui/components/select"
import { Skeleton } from "@circulo/ui/components/skeleton"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@circulo/ui/components/tabs"
import { useAtomValue } from "jotai"
import {
	AlertCircleIcon,
	BookOpenIcon,
	ChevronRightIcon,
	DownloadIcon,
	ExternalLinkIcon,
	FolderIcon,
	GlobeIcon,
	LoaderCircleIcon,
	RefreshCwIcon,
	SearchIcon,
	CheckIcon,
	Trash2Icon,
} from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import type { InstalledSkill } from "../../preload/api"
import { discoveryProjectsAtom } from "../../atoms/discovery"
import { useInstalledSkills } from "../../hooks/use-installed-skills"
import { useSkillDiscovery, type DiscoveredSkill } from "../../hooks/use-skill-discovery"
import { useSkillInstall } from "../../hooks/use-skill-install"
import { SettingsRow } from "./settings-row"
import { SettingsSection } from "./settings-section"

// ============================================================
// Sub-tab identifiers
// ============================================================

type SkillSubTab = "installed" | "discover"

// ============================================================
// Formatting
// ============================================================

function formatInstalls(n: number): string {
	if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1).replace(/\.0$/, "")}M`
	if (n >= 1_000) return `${(n / 1_000).toFixed(1).replace(/\.0$/, "")}K`
	return n.toLocaleString()
}

// ============================================================
// Main component
// ============================================================

export function SkillSettings() {
	const [subTab, setSubTab] = useState<SkillSubTab>("installed")

	return (
		<div className="h-full flex flex-col min-h-0 space-y-4">
			<h2 className="text-xl font-semibold">Skills</h2>

			<Tabs value={subTab} onValueChange={(v) => setSubTab(v as SkillSubTab)} orientation="horizontal">
				<div className="shrink-0 pt-2 pb-2">
					<TabsList variant="default">
						<TabsTrigger value="installed">
							<BookOpenIcon aria-hidden="true" className="size-4" />
							Installed
						</TabsTrigger>
						<TabsTrigger value="discover">
							<GlobeIcon aria-hidden="true" className="size-4" />
							Discover
						</TabsTrigger>
					</TabsList>
				</div>
				<TabsContent value="installed" className="flex-1 min-h-0 overflow-y-auto">
					<InstalledView />
				</TabsContent>
				<TabsContent value="discover" className="flex-1 min-h-0 overflow-y-auto">
					<DiscoverView />
				</TabsContent>
			</Tabs>
		</div>
	)
}

// ============================================================
// Installed view — scope selector + cards per scope
// ============================================================

type InstalledScope = "all" | "global" | "cursor" | "global-agents"

interface ProjectScope {
	label: string
	dir: string
	count: number
}

function InstalledView() {
	const { grouped, projectsMap, loading, error, refresh, diagnostics } = useInstalledSkills()
	const [scope, setScope] = useState<InstalledScope | string>("all")
	const [selectedSkill, setSelectedSkill] = useState<InstalledSkill | null>(null)
	const allProjects = useAtomValue(discoveryProjectsAtom)

	const projectScopes: ProjectScope[] = useMemo(() => {
		const list: ProjectScope[] = []
		for (const [dir, skills] of projectsMap) {
			const proj = allProjects.find((p) => (p as { worktree?: string }).worktree === dir)
			list.push({
				label: proj ? (proj as { name?: string }).name ?? dir.split("/").pop() ?? dir : dir.split("/").pop() ?? dir,
				dir,
				count: skills.length,
			})
		}
		return list.sort((a, b) => b.count - a.count)
	}, [projectsMap, allProjects])

	const totalCount = grouped.global.length + grouped.project.length + grouped.cursor.length

	if (loading) {
		return (
			<div className="space-y-3 pt-2">
				<Skeleton className="h-12 w-full" />
				<Skeleton className="h-12 w-full" />
				<Skeleton className="h-12 w-full" />
			</div>
		)
	}

	if (error) {
		return (
			<div className="space-y-3 pt-2">
				<div className="flex items-center gap-2 text-destructive">
					<AlertCircleIcon aria-hidden="true" className="size-4" />
					<span className="text-sm">{error}</span>
				</div>
				<Button variant="outline" size="sm" onClick={refresh}>
					<RefreshCwIcon aria-hidden="true" className="size-4" />
					Retry
				</Button>
			</div>
		)
	}

	return (
		<div className="space-y-4">
			<div className="flex items-center justify-between">
				<p className="text-sm text-muted-foreground">
					{totalCount} skill{totalCount !== 1 ? "s" : ""} installed
				</p>
				<Button variant="ghost" size="sm" onClick={refresh}>
					<RefreshCwIcon aria-hidden="true" className="size-4" />
					Refresh
				</Button>
			</div>

			{totalCount === 0 ? (
				<div className="py-6 text-center space-y-2">
					<p className="text-sm text-muted-foreground">No skills installed</p>
					{diagnostics && (
						<div className="text-xs text-muted-foreground space-y-0.5">
							<p>Scanned: {diagnostics.globalDir}</p>
							<p>Scanned: {diagnostics.agentsDir}</p>
							<p>Scanned: {diagnostics.cursorDir}</p>
							{diagnostics.projectDirs.length > 0 && (
								<p>Scanned {diagnostics.projectDirs.length} project director{diagnostics.projectDirs.length !== 1 ? "ies" : "y"}</p>
							)}
						</div>
					)}
					<p className="text-xs text-muted-foreground pt-1">
						Install skills from the Discover tab or run{" "}
						<code className="bg-muted px-1 rounded text-xs">npx skills add &lt;owner/repo&gt;</code>
					</p>
					<Button
						variant="link"
						size="sm"
						onClick={() => {
							const tabTrigger = document.querySelector('[data-value="discover"]') as HTMLElement | null
							tabTrigger?.click()
						}}
					>
						Discover skills on skills.sh
					</Button>
				</div>
			) : selectedSkill ? (
				<InstalledDetailPanel
					skill={selectedSkill}
					projectName={
						selectedSkill.project
							? projectScopes.find((s) => s.dir === selectedSkill.project)?.label
							: undefined
					}
					onBack={() => setSelectedSkill(null)}
				/>
			) : scope === "all" ? (
				<div className="space-y-2">
					{/* Global scope card */}
					{grouped.global.length > 0 && (
						<ScopeCard
							icon={<GlobeIcon aria-hidden="true" className="size-5" />}
							label="Global"
							subtitle="~/.config/opencode/skills/ & ~/.agents/skills/"
							count={grouped.global.length}
							onClick={() => setScope("global")}
						/>
					)}

					{/* Cursor scope card */}
					{grouped.cursor.length > 0 && (
						<ScopeCard
							icon={<ArrowRightIcon aria-hidden="true" className="size-5" />}
							label="Cursor (migrated)"
							subtitle="~/.cursor/skills/"
							count={grouped.cursor.length}
							onClick={() => setScope("cursor")}
						/>
					)}

					{/* Per-project scope cards */}
					{projectScopes.map((ps) => (
						<ScopeCard
							key={ps.dir}
							icon={<FolderIcon aria-hidden="true" className="size-5" />}
							label={ps.label}
							subtitle={ps.dir}
							count={ps.count}
							onClick={() => setScope(ps.dir)}
						/>
					))}

					{!grouped.global.length && !grouped.cursor.length && !projectScopes.length && (
						<p className="text-sm text-muted-foreground py-4">No skills installed</p>
					)}
				</div>
			) : (
				/* Single scope detail view */
				<ScopedSkillList
					skills={
						scope === "global"
							? grouped.global
							: scope === "cursor"
								? grouped.cursor
								: projectsMap.get(scope) ?? []
					}
					projectScopes={projectScopes}
					onSelect={setSelectedSkill}
					onDelete={async (s) => {
						if ("circulo" in window) {
							await window.circulo.skills.remove(s.location)
							refresh()
						}
					}}
					onBack={() => setScope("all")}
				/>
			)}
		</div>
	)
}

// ============================================================
// Scope card (All skills overview)
// ============================================================

function ScopeCard({
	icon,
	label,
	subtitle,
	count,
	onClick,
}: {
	icon: React.ReactNode
	label: string
	subtitle: string
	count: number
	onClick: () => void
}) {
	return (
		<button
			type="button"
			className="flex items-center gap-3 w-full rounded-lg border border-border p-3 text-left hover:bg-accent transition-colors"
			onClick={onClick}
		>
			<div className="text-muted-foreground shrink-0">{icon}</div>
			<div className="flex-1 min-w-0">
				<p className="text-sm font-medium">{label}</p>
				<p className="text-xs text-muted-foreground truncate">{subtitle}</p>
			</div>
			<Badge variant="secondary">{count}</Badge>
			<ChevronRightIcon aria-hidden="true" className="size-4 text-muted-foreground shrink-0" />
		</button>
	)
}

// ============================================================
// Skill row (for scope detail lists)
// ============================================================

function SkillRow({
	skill,
	projectLabel,
	onSelect,
	onDelete,
}: {
	skill: InstalledSkill
	projectLabel?: string
	onSelect: (skill: InstalledSkill) => void
	onDelete: (skill: InstalledSkill) => void
}) {
	return (
		<div className="flex items-center justify-between gap-3 px-4 py-2.5">
			<div className="flex-1 min-w-0">
				<p className="text-sm font-medium truncate">{skill.name}</p>
				<p className="text-xs text-muted-foreground truncate">
					{skill.description}
					{projectLabel && <span className="ml-1">· {projectLabel}</span>}
				</p>
			</div>
			<div className="flex items-center gap-1 shrink-0">
				<Button variant="ghost" size="sm" onClick={() => onSelect(skill)}>
					Details
				</Button>
				<Button
					variant="ghost"
					size="icon-sm"
					onClick={() => {
						if (confirm(`Delete "${skill.name}"? This removes the skill directory from disk.`)) {
							onDelete(skill)
						}
					}}
				>
					<Trash2Icon aria-hidden="true" className="size-4 text-muted-foreground hover:text-destructive" />
				</Button>
			</div>
		</div>
	)
}

// ============================================================
// Generic skills list (reused for Installed scope detail)
// ============================================================

function SkillsList<T>({
	skills,
	onSelect,
	renderItem,
}: {
	skills: T[]
	onSelect: (skill: T) => void
	renderItem: (skill: T) => React.ReactNode
}) {
	return (
		<div className="divide-y divide-border rounded-lg border border-border">
			{skills.map((skill, i) => (
				<div key={(skill as { location?: string; id?: string }).location ?? (skill as { id?: string }).id ?? i}>
					{renderItem(skill)}
				</div>
			))}
		</div>
	)
}

// ============================================================
// Scoped skill list with pagination (Installed view)
// ============================================================

function ScopedSkillList({
	skills,
	projectScopes,
	onSelect,
	onDelete,
	onBack,
}: {
	skills: InstalledSkill[]
	projectScopes: ProjectScope[]
	onSelect: (skill: InstalledSkill) => void
	onDelete: (skill: InstalledSkill) => void
	onBack: () => void
}) {
	const { page, setPage, paginated, totalPages } = usePagination(skills, 7)

	return (
		<div className="space-y-3">
			<Button variant="ghost" size="sm" onClick={onBack}>
				← All scopes
			</Button>
			<SkillsList
				skills={paginated}
				onSelect={onSelect}
				renderItem={(skill) => (
					<SkillRow
						skill={skill}
						projectLabel={projectScopes.find((s) => s.dir === skill.project)?.label}
						onSelect={onSelect}
						onDelete={onDelete}
					/>
				)}
			/>
			<Paginator page={page} totalPages={totalPages} onPageChange={setPage} />
		</div>
	)
}

// ============================================================
// Paginator component
// ============================================================

function Paginator({
	page,
	totalPages,
	onPageChange,
}: {
	page: number
	totalPages: number
	onPageChange: (page: number) => void
}) {
	if (totalPages <= 1) return null

	return (
		<div className="flex items-center justify-center gap-3 pt-3">
			<Button
				variant="outline"
				size="sm"
				disabled={page <= 1}
				onClick={() => onPageChange(page - 1)}
			>
				← Prev
			</Button>
			<span className="text-xs text-muted-foreground">
				Page {page} of {totalPages}
			</span>
			<Button
				variant="outline"
				size="sm"
				disabled={page >= totalPages}
				onClick={() => onPageChange(page + 1)}
			>
				Next →
			</Button>
		</div>
	)
}

// ============================================================
// Pagination hook
// ============================================================

function usePagination<T>(items: T[], perPage: number) {
	const [page, setPage] = useState(1)
	const totalPages = Math.max(1, Math.ceil(items.length / perPage))
	const safePage = Math.min(page, totalPages)
	const start = (safePage - 1) * perPage
	const paginated = items.slice(start, start + perPage)

	return { page: safePage, setPage, paginated, totalPages }
}

// ============================================================
// Installed skill detail panel
// ============================================================

function InstalledDetailPanel({
	skill,
	projectName,
	onBack,
}: {
	skill: InstalledSkill
	projectName?: string
	onBack: () => void
}) {
	return (
		<SettingsSection title={skill.name}>
			<div className="space-y-3 px-4 py-3">
				<div>
					<p className="text-sm font-medium">Description</p>
					<p className="text-sm text-muted-foreground">{skill.description}</p>
				</div>
				<div>
					<p className="text-sm font-medium">Location</p>
					<p className="text-sm text-muted-foreground font-mono text-xs break-all">
						{skill.location}
					</p>
				</div>
				<div className="flex items-center gap-3">
					<div>
						<p className="text-sm font-medium">Origin</p>
						<Badge variant="outline">{skill.origin}</Badge>
					</div>
					{projectName && (
						<div>
							<p className="text-sm font-medium">Project</p>
							<p className="text-sm text-muted-foreground">{projectName}</p>
						</div>
					)}
				</div>
				<Button variant="ghost" size="sm" onClick={onBack}>
					← Back
				</Button>
			</div>
		</SettingsSection>
	)
}

// ============================================================
// Helpers
// ============================================================

function useProjectDirs(): { dir: string; label: string }[] {
	const projects = useAtomValue(discoveryProjectsAtom)
	const dirs = useMemo(() => {
		const result = [
			...new Map(
				projects
					.filter(
						(p) =>
							"worktree" in p &&
							typeof (p as { worktree: unknown }).worktree === "string",
					)
					.map((p) => {
						const d = (p as { worktree: string }).worktree
						const rawName = (p as { name?: string }).name
						const n = rawName?.trim() || d.split("/").pop() || d
						return [d, { dir: d, label: n }] as const
					}),
			).values(),
		]
		return result
	}, [projects])
	return dirs
}

// ============================================================
// Discover view
// ============================================================

function DiscoverView() {
	const { skills, loading, error, lastQuery, search, fetchInitial, openSkillsSh } = useSkillDiscovery()
	const { state, error: installStateError, install, reset } = useSkillInstall()
	const [searchInput, setSearchInput] = useState("")
	const [selectedSkill, setSelectedSkill] = useState<DiscoveredSkill | null>(null)
	const [installTarget, setInstallTarget] = useState<string>("global")
	const projectDirs = useProjectDirs()
	const labelToDir = useMemo(() => {
		const map = new Map<string, string>()
		for (const { dir, label } of projectDirs) {
			map.set(label, dir)
		}
		return map
	}, [projectDirs])
	const initialLoaded = useRef(false)

	useEffect(() => {
		if (!initialLoaded.current) {
			initialLoaded.current = true
			fetchInitial()
		}
	}, [fetchInitial])

	const handleSearch = (e: React.FormEvent) => {
		e.preventDefault()
		if (searchInput.trim().length >= 2) {
			search(searchInput.trim())
		}
	}

	const handleInstall = (ownerRepoSource: string, skillName: string) => {
		const resolved = installTarget === "global" ? undefined : labelToDir.get(installTarget) ?? installTarget
		install(ownerRepoSource, skillName, resolved)
	}

	return (
		<div className="space-y-4">
			<div className="pb-2">
				<form onSubmit={handleSearch} className="flex gap-2">
					<Input
						placeholder="Search skills.sh..."
						value={searchInput}
						onChange={(e) => {
							setSearchInput(e.target.value)
							if (e.target.value.trim().length === 0) {
								fetchInitial()
							}
						}}
						className="flex-1"
					/>
					<Button type="submit" variant="secondary" size="sm" disabled={searchInput.trim().length < 2}>
						<SearchIcon aria-hidden="true" className="size-4" />
					</Button>
					<Button variant="outline" size="sm" onClick={openSkillsSh} title="Open skills.sh in browser">
						<ExternalLinkIcon aria-hidden="true" className="size-4" />
					</Button>
				</form>
			</div>

			{loading && (
				<div className="space-y-3">
					<Skeleton className="h-24 w-full" />
					<Skeleton className="h-24 w-full" />
					<Skeleton className="h-24 w-full" />
				</div>
			)}

			{error && (
				<div className="flex items-center gap-2 text-destructive">
					<AlertCircleIcon aria-hidden="true" className="size-4" />
					<span className="text-sm">{error}</span>
					<Button variant="link" size="sm" onClick={() => search(lastQuery || "development")}>
						Retry
					</Button>
				</div>
			)}

			{!loading && !error && skills.length === 0 && (
				<p className="text-sm text-muted-foreground py-4">
					{lastQuery ? `No skills found for "${lastQuery}"` : "Search for skills or open skills.sh"}
				</p>
			)}

			{selectedSkill ? (
				<SkillDetailPanel
					skill={selectedSkill}
					installState={state}
					installError={installStateError}
					installTarget={installTarget}
					projectDirs={projectDirs}
					onInstall={() => handleInstall(selectedSkill.source)}
					onTargetChange={setInstallTarget}
					onBack={() => {
						setSelectedSkill(null)
						reset()
					}}
				/>
			) : (
				<DiscoverSkillsList skills={skills} onSelect={setSelectedSkill} />
			)}

			<Dialog
				open={selectedSkill !== null}
				onOpenChange={(open) => {
					if (!open) {
						setSelectedSkill(null)
						reset()
					}
				}}
			>
				{selectedSkill && (
					<DialogContent showCloseButton={false}>
						<SkillDetailPanel
							skill={selectedSkill}
							installState={state}
							installError={installStateError}
							installTarget={installTarget}
							projectDirs={projectDirs}
							onInstall={() => handleInstall(selectedSkill.source, selectedSkill.name)}
							onTargetChange={setInstallTarget}
							onBack={() => {
								setSelectedSkill(null)
								reset()
							}}
						/>
					</DialogContent>
				)}
			</Dialog>
		</div>
	)
}

// ============================================================
// Discover skills list with paginator
// ============================================================

function DiscoverSkillsList({
	skills,
	onSelect,
}: {
	skills: DiscoveredSkill[]
	onSelect: (skill: DiscoveredSkill) => void
}) {
	const { page, setPage, paginated, totalPages } = usePagination(skills, 7)

	return (
		<div>
			<div className="space-y-2">
				{paginated.map((skill) => (
					<div
						key={skill.id}
						className="rounded-lg border border-border p-4 hover:bg-accent/50 transition-colors"
					>
						<div className="flex items-start justify-between gap-3">
							<div className="flex-1 min-w-0">
								<p className="text-sm font-semibold">{skill.name}</p>
								<p className="text-xs text-muted-foreground">{skill.source}</p>
								<div className="flex items-center gap-2 mt-2">
									<Badge variant="secondary" className="text-xs font-normal">
										<DownloadIcon aria-hidden="true" className="size-3 mr-1" />
										{formatInstalls(skill.installs)} installs
									</Badge>
									<a
										href={skill.url}
										target="_blank"
										rel="noopener noreferrer"
										className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
										onClick={(e) => e.stopPropagation()}
									>
										<ExternalLinkIcon aria-hidden="true" className="size-3" />
										skills.sh
									</a>
								</div>
							</div>
							<Button
								size="sm"
								variant="outline"
								onClick={() => onSelect(skill)}
								className="shrink-0"
							>
								<DownloadIcon aria-hidden="true" className="size-3.5" />
								Install
							</Button>
						</div>
					</div>
				))}
			</div>
			<Paginator page={page} totalPages={totalPages} onPageChange={setPage} />
		</div>
	)
}

// ============================================================
// Skill detail panel (used by Discover for install target)
// ============================================================

function SkillDetailPanel({
	skill,
	installState,
	installError,
	installTarget,
	projectDirs,
	onInstall,
	onTargetChange,
	onBack,
}: {
	skill: DiscoveredSkill
	installState: string
	installError: string | null
	installTarget: string
	projectDirs: { dir: string; label: string }[]
	onInstall: () => void
	onTargetChange: (value: string) => void
	onBack: () => void
}) {
	return (
		<div className="space-y-3">
			<h3 className="text-lg font-semibold">{skill.name}</h3>
			<div>
				<p className="text-sm font-medium">Source</p>
				<p className="text-sm text-muted-foreground">{skill.source}</p>
			</div>
			<div>
				<p className="text-sm font-medium">Installs</p>
				<p className="text-sm text-muted-foreground">{formatInstalls(skill.installs)}</p>
			</div>
			<a
				href={skill.url}
				target="_blank"
				rel="noopener noreferrer"
				className="inline-flex items-center gap-1 text-sm text-primary hover:underline"
			>
				<ExternalLinkIcon aria-hidden="true" className="size-3" />
				View on skills.sh
			</a>

			<div className="space-y-2 pt-2">
				<div>
					<p className="text-sm font-medium mb-1">Install to</p>
						<Select value={installTarget} onValueChange={(v) => v && onTargetChange(v)}>
							<SelectTrigger className="w-full truncate">
								<SelectValue placeholder="Select destination..." />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value="global">
									<GlobeIcon aria-hidden="true" className="size-3 shrink-0" />
									Global (~/.config/opencode/skills/)
								</SelectItem>
								{projectDirs.map(({ dir, label }) => (
									<SelectItem key={dir} value={label}>
										<FolderIcon aria-hidden="true" className="size-3 shrink-0" />
										{label}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
				</div>

				{installState === "idle" && (
					<Button onClick={onInstall} className="w-full">
						<DownloadIcon aria-hidden="true" className="size-4" />
						Install
					</Button>
				)}
				{installState === "installing" && (
					<Button disabled className="w-full">
						<LoaderCircleIcon aria-hidden="true" className="size-4 animate-spin" />
						Installing...
					</Button>
				)}
				{installState === "success" && (
					<Button disabled className="w-full" variant="outline">
						<CheckIcon aria-hidden="true" className="size-4" />
						Installed
					</Button>
				)}
				{installState === "error" && (
					<div className="space-y-2">
						<p className="text-sm text-destructive">{installError}</p>
						<Button onClick={onInstall} className="w-full">
							Retry
						</Button>
					</div>
				)}
			</div>

			<Button variant="ghost" size="sm" onClick={onBack}>
				{installState === "success" ? "Close" : "Cancel"}
			</Button>
		</div>
	)
}

// ============================================================
// Arrow right icon (SVG)
// ============================================================

function ArrowRightIcon(props: React.SVGProps<SVGSVGElement>) {
	return (
		<svg
			xmlns="http://www.w3.org/2000/svg"
			width="24"
			height="24"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
			{...props}
		>
			<path d="M5 12h14" />
			<path d="m12 5 7 7-7 7" />
		</svg>
	)
}
