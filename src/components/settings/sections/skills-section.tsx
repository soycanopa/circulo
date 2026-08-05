import { useAtomValue, useSetAtom } from "jotai"
import { Loader2, Search, ShieldCheck, Trash2 } from "lucide-react"
import { useCallback, useEffect, useState } from "react"
import {
	SectionHeader,
	SettingRow,
} from "@/components/settings/sections/section-ui"
import { Input } from "@/components/ui/input"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import {
	deleteSkill,
	installSkill,
	listSkills,
	searchSkills,
	setVercelOidcToken,
} from "@/lib/tauri"
import { appSettingsAtom, projectPathAtom } from "@/stores/atoms"
import type {
	InstalledSkill,
	SkillSearchResult,
} from "@/types/acp"

export function SkillsSection() {
	const projectPath = useAtomValue(projectPathAtom)
	const appSettings = useAtomValue(appSettingsAtom)
	const setAppSettings = useSetAtom(appSettingsAtom)
	const [query, setQuery] = useState("")
	const [results, setResults] = useState<SkillSearchResult[]>([])
	const [installed, setInstalled] = useState<InstalledSkill[]>([])
	const [searching, setSearching] = useState(false)
	const [busy, setBusy] = useState(false)
	const [searchError, setSearchError] = useState<string | null>(null)
	const [searchMode, setSearchMode] = useState<string | null>(null)
	const [targets, setTargets] = useState<Record<string, string>>({})
	const [tokenInput, setTokenInput] = useState(
		appSettings?.vercelOidcToken ?? "",
	)
	const [savingToken, setSavingToken] = useState(false)

	const oidcToken = appSettings?.vercelOidcToken ?? null

	const refreshInstalled = useCallback(async () => {
		try {
			const res = await listSkills(projectPath)
			setInstalled(res.skills)
		} catch {
			setInstalled([])
		}
	}, [projectPath])

	useEffect(() => {
		void refreshInstalled()
	}, [refreshInstalled])

	useEffect(() => {
		if (!query.trim()) {
			setResults([])
			setSearchError(null)
			setSearchMode(null)
			return
		}
		let cancelled = false
		const delay = setTimeout(() => {
			setSearching(true)
			void searchSkills(query.trim(), 10, oidcToken)
				.then((res) => {
					if (cancelled) return
					setResults(res.skills)
					setSearchError(res.error)
					setSearchMode(res.mode)
				})
				.catch((err) => {
					if (cancelled) return
					setResults([])
					setSearchError(
						err instanceof Error ? err.message : "Search failed",
					)
				})
				.finally(() => {
					if (!cancelled) setSearching(false)
				})
		}, 350)
		return () => {
			cancelled = true
			clearTimeout(delay)
		}
	}, [query, oidcToken])

	async function handleSaveToken() {
		setSavingToken(true)
		setSearchError(null)
		try {
			const settings = await setVercelOidcToken(tokenInput.trim() || null)
			setAppSettings(settings)
		} catch (err) {
			setSearchError(err instanceof Error ? err.message : "Could not save token")
		} finally {
			setSavingToken(false)
		}
	}

	async function handleInstall(skill: SkillSearchResult) {
		const target = targets[skill.id] ?? "project"
		setBusy(true)
		setSearchError(null)
		try {
			await installSkill(skill.skillId, skill.source, target, projectPath, skill.id, oidcToken)
			await refreshInstalled()
		} catch (err) {
			setSearchError(err instanceof Error ? err.message : "Install failed")
		} finally {
			setBusy(false)
		}
	}

	async function handleDelete(skill: InstalledSkill) {
		setBusy(true)
		try {
			await deleteSkill(skill.name, skill.scope, projectPath)
			await refreshInstalled()
		} catch (err) {
			setSearchError(err instanceof Error ? err.message : "Delete failed")
		} finally {
			setBusy(false)
		}
	}

	return (
		<div>
			<SectionHeader
				title="Skills"
				description="Search the skills.sh directory and install SKILL.md skills to the project (.opencode/skills) or globally (~/.config/opencode/skills)."
			/>

			{searchError ? (
				<div className="mb-4 rounded-lg border border-amber-500/25 bg-amber-500/5 px-3.5 py-2 text-[11px] text-amber-200/90">
					{searchError}
				</div>
			) : null}

			{searchMode ? (
				<div
					className={`mb-4 flex items-center gap-2 rounded-lg border px-3.5 py-2 text-[11px] ${
						searchMode === "authenticated"
							? "border-emerald-500/25 bg-emerald-500/5 text-emerald-200/90"
							: "border-border bg-black/20 text-muted"
					}`}
				>
					<ShieldCheck className="size-3.5" />
					{searchMode === "authenticated"
						? "Searching via the official skills.sh API (Vercel OIDC)."
						: "Searching via the public skills.sh endpoint (no token configured)."}
				</div>
			) : null}

			<div className="relative">
				<Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted" />
				<Input
					value={query}
					onChange={(e) => setQuery(e.target.value)}
					placeholder="Search skills… (e.g. tdd, react, go)"
					className="pl-8"
				/>
			</div>

			{searching ? (
				<div className="mt-3 flex items-center gap-2 text-[11px] text-muted">
					<Loader2 className="size-3 animate-spin" /> Searching skills.sh…
				</div>
			) : null}

			{results.length > 0 ? (
				<div className="mt-3 space-y-2">
					{results.map((skill) => {
						const target = targets[skill.id] ?? "project"
						return (
							<div
								key={skill.id}
								className="flex items-center justify-between gap-3 rounded-lg border border-border bg-black/20 px-3.5 py-2.5"
							>
								<div className="min-w-0 flex-1">
									<div className="flex items-center gap-2 text-sm text-fg">
										{skill.name}
										<span className="rounded bg-white/10 px-1.5 py-0.5 font-mono text-[10px] text-muted">
											{skill.installs.toLocaleString()} installs
										</span>
										{skill.sourceType ? (
											<span className="rounded bg-white/5 px-1.5 py-0.5 font-mono text-[10px] text-muted">
												{skill.sourceType}
											</span>
										) : null}
									</div>
									<div className="mt-0.5 truncate text-[10px] text-muted">
										{skill.source}
									</div>
									{skill.description ? (
										<div className="mt-0.5 line-clamp-2 text-[11px] text-muted">
											{skill.description}
										</div>
									) : null}
								</div>
								<div className="flex shrink-0 items-center gap-2">
									<Select
										value={target}
										onValueChange={(value) =>
											setTargets((prev) => ({
												...prev,
												[skill.id]: value,
											}))
										}
									>
										<SelectTrigger className="h-7 w-24">
											<SelectValue />
										</SelectTrigger>
										<SelectContent>
											<SelectItem value="project">project</SelectItem>
											<SelectItem value="global">global</SelectItem>
										</SelectContent>
									</Select>
									<button
										type="button"
										onClick={() => void handleInstall(skill)}
										disabled={busy}
										className="rounded-md border border-border px-2.5 py-1 text-[11px] text-fg transition hover:bg-white/5 disabled:opacity-50"
									>
										Install
									</button>
								</div>
							</div>
						)
					})}
				</div>
			) : null}

			<div className="mt-6 space-y-3">
				<div className="rounded-lg border border-border bg-black/20 px-3.5 py-3">
					<div className="text-sm text-fg">Vercel OIDC token (optional)</div>
					<div className="mt-0.5 text-[11px] leading-snug text-muted">
						Enables the official skills.sh API: semantic search and direct
						SKILL.md download (handles well-known sources, avoids GitHub rate
						limits). Get a token from your Vercel project's OIDC Federation
						settings — it is stored in Circulo settings, never sent anywhere
						else.
					</div>
					<div className="mt-2 flex items-center gap-2">
						<Input
							value={tokenInput}
							onChange={(e) => setTokenInput(e.target.value)}
							placeholder="VERCEL_OIDC_TOKEN"
							type="password"
							className="flex-1"
						/>
						<button
							type="button"
							onClick={() => void handleSaveToken()}
							disabled={savingToken}
							className="shrink-0 rounded-md border border-border px-2.5 py-1 text-[11px] text-fg transition hover:bg-white/5 disabled:opacity-50"
						>
							{savingToken ? "Saving…" : "Save"}
						</button>
					</div>
				</div>
			</div>

			<div className="mt-6">
				<div className="mb-2 text-[11px] font-medium uppercase tracking-wide text-muted">
					Installed
				</div>
				{installed.length === 0 ? (
					<p className="rounded-lg border border-dashed border-border px-3.5 py-4 text-center text-[11px] text-muted">
						No skills installed yet.
					</p>
				) : (
					<div className="space-y-2">
						{installed.map((skill) => (
							<SettingRow
								key={`${skill.scope}:${skill.name}`}
								label={
									<span className="flex items-center gap-2">
										{skill.name}
										<span className="rounded bg-white/10 px-1.5 py-0.5 text-[10px] text-muted">
											{skill.scope}
										</span>
									</span>
								}
								description={
									skill.description || (
										<code className="block truncate font-mono text-[10px] text-muted">
											{skill.path}
										</code>
									)
								}
								control={
									<button
										type="button"
										onClick={() => void handleDelete(skill)}
										disabled={busy}
										className="rounded p-1 text-muted transition hover:bg-red-500/10 hover:text-red-300 disabled:opacity-50"
										aria-label={`Delete ${skill.name}`}
									>
										<Trash2 className="size-3.5" />
									</button>
								}
							/>
						))}
					</div>
				)}
			</div>
		</div>
	)
}
