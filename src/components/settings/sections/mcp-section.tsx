import { useAtomValue } from "jotai"
import { Bot, Loader2, Plus, Plug, RefreshCw, Trash2, X } from "lucide-react"
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
import { Switch } from "@/components/ui/switch"
import {
	deleteMcpServer,
	getCirculoMcpStatus,
	getMcpServers,
	importMcp,
	listMcpImports,
	setMcpServerState,
	upsertMcpServer,
	validateMcpServer,
} from "@/lib/tauri"
import { capabilitiesAtom, projectPathAtom } from "@/stores/atoms"
import type {
	CirculoMcpStatus,
	ManagedMcpServer,
	McpImportCandidate,
	McpServerKind,
} from "@/types/acp"

interface McpPreset {
	id: string
	label: string
	kind: McpServerKind
	command: string
	args: string[]
	env: { name: string; value: string }[]
	hint: string
}

const MCP_PRESETS: McpPreset[] = [
	{
		id: "filesystem",
		label: "Filesystem",
		kind: "stdio",
		command: "npx",
		args: ["-y", "@modelcontextprotocol/server-filesystem"],
		env: [],
		hint: "Read, search and edit files in a folder.",
	},
	{
		id: "github",
		label: "GitHub",
		kind: "stdio",
		command: "npx",
		args: ["-y", "@modelcontextprotocol/server-github"],
		env: [{ name: "GITHUB_PERSONAL_ACCESS_TOKEN", value: "" }],
		hint: "Issues, PRs, repos and code search.",
	},
	{
		id: "brave-search",
		label: "Brave Search",
		kind: "stdio",
		command: "npx",
		args: ["-y", "@modelcontextprotocol/server-brave-search"],
		env: [{ name: "BRAVE_API_KEY", value: "" }],
		hint: "Web search via the Brave Search API.",
	},
	{
		id: "sequential-thinking",
		label: "Sequential Thinking",
		kind: "stdio",
		command: "npx",
		args: ["-y", "@modelcontextprotocol/server-sequential-thinking"],
		env: [],
		hint: "Structured problem-solving steps.",
	},
	{
		id: "fetch",
		label: "Fetch",
		kind: "stdio",
		command: "npx",
		args: ["-y", "mcp-server-fetch"],
		env: [],
		hint: "Fetch a URL and convert it to markdown.",
	},
]

function slugify(name: string): string {
	return (
		name
			.toLowerCase()
			.trim()
			.replace(/[^a-z0-9]+/g, "-")
			.replace(/^-+|-+$/g, "") || "server"
	)
}

const EMPTY_FORM = {
	name: "",
	kind: "stdio" as McpServerKind,
	command: "",
	args: "",
	env: "",
	autoLoad: false,
}

function CapBadge({
	label,
	ok,
	empirical,
}: {
	label: string
	ok: boolean
	empirical?: boolean
}) {
	return (
		<span
			className={`inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] ${
				ok
					? "bg-emerald-500/15 text-emerald-200"
					: "bg-white/5 text-muted line-through"
			}`}
		>
			{ok ? "✓" : "—"} {label}
			{empirical ? (
				<span className="opacity-60">(runtime)</span>
			) : null}
		</span>
	)
}

export function McpSection() {
	const projectPath = useAtomValue(projectPathAtom)
	const capabilities = useAtomValue(capabilitiesAtom)
	const [servers, setServers] = useState<ManagedMcpServer[]>([])
	const [status, setStatus] = useState<CirculoMcpStatus | null>(null)
	const [imports, setImports] = useState<McpImportCandidate[]>([])
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const [adding, setAdding] = useState(false)
	const [form, setForm] = useState(EMPTY_FORM)
	const [validation, setValidation] = useState<{
		state: "idle" | "testing" | "ok" | "failed"
		tools: string[]
		message: string | null
	}>({ state: "idle", tools: [], message: null })

	const refresh = useCallback(async () => {
		const [serversRes, statusRes] = await Promise.all([
			getMcpServers(),
			getCirculoMcpStatus(),
		])
		setServers(serversRes)
		setStatus(statusRes)
	}, [])

	const refreshImports = useCallback(async () => {
		if (!projectPath) {
			setImports([])
			return
		}
		try {
			setImports(await listMcpImports(projectPath))
		} catch {
			setImports([])
		}
	}, [projectPath])

	useEffect(() => {
		void refresh().catch(() => setError("Could not load MCP servers"))
	}, [refresh])

	useEffect(() => {
		void refreshImports()
	}, [refreshImports])

	async function handleToggleState(
		server: ManagedMcpServer,
		enabled: boolean,
		autoLoad: boolean,
	) {
		setBusy(true)
		setError(null)
		try {
			setServers(await setMcpServerState(server.id, enabled, autoLoad))
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		} finally {
			setBusy(false)
		}
	}

	async function handleDelete(id: string) {
		setBusy(true)
		setError(null)
		try {
			setServers(await deleteMcpServer(id))
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		} finally {
			setBusy(false)
		}
	}

	function presetForm(preset: McpPreset) {
		setForm({
			name: preset.label,
			kind: preset.kind,
			command: preset.command,
			args: preset.args.join(" "),
			env: preset.env
				.map((e) => (e.value ? `${e.name}=${e.value}` : e.name))
				.join("\n"),
			autoLoad: false,
		})
		setValidation({ state: "idle", tools: [], message: null })
		setAdding(true)
	}

	async function handleValidate() {
		const built = buildServer(form)
		if (!built) return
		setValidation({ state: "testing", tools: [], message: null })
		const result = await validateMcpServer(built)
		setValidation({
			state: result.ok ? "ok" : "failed",
			tools: result.tools,
			message: result.error,
		})
	}

	async function handleSave() {
		const built = buildServer(form)
		if (!built) return
		setBusy(true)
		setError(null)
		try {
			setServers(await upsertMcpServer(built))
			setForm(EMPTY_FORM)
			setAdding(false)
			setValidation({ state: "idle", tools: [], message: null })
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		} finally {
			setBusy(false)
		}
	}

	async function handleImport(candidate: McpImportCandidate) {
		if (!projectPath) return
		setBusy(true)
		setError(null)
		try {
			setServers(await importMcp(projectPath, candidate.id))
			await refreshImports()
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		} finally {
			setBusy(false)
		}
	}

	return (
		<div>
			<SectionHeader
				title="MCP servers"
				description="Circulo injects its orchestrator (circulo-mcp) into every session. Other servers load on-demand via /mcp or auto-load with their full tool catalogue."
			/>

			{status ? (
				<div
					className={`mb-4 rounded-lg border px-3.5 py-2.5 text-[11px] leading-snug ${
						status.available
							? "border-emerald-500/25 bg-emerald-500/5 text-emerald-200/90"
							: "border-amber-500/25 bg-amber-500/5 text-amber-200/90"
					}`}
				>
					<div className="flex items-center gap-2 font-medium">
						<Plug className="size-3.5" />
						{status.available
							? "Orchestrator circulo-mcp available"
							: "Orchestrator binary not found — MCP orchestration is disabled"}
					</div>
					<div className="mt-0.5 truncate font-mono text-[10px] opacity-70">
						{status.path ?? status.registryPath}
					</div>
				</div>
			) : null}

			{capabilities ? (
				<div className="mb-4 rounded-lg border border-border bg-black/20 px-3.5 py-2.5 text-[11px] leading-snug">
					<div className="mb-1.5 flex items-center gap-2 font-medium text-fg">
						<Bot className="size-3.5" />
						Agent capabilities
					</div>
					<div className="flex flex-wrap gap-1.5">
						<CapBadge label="MCP stdio" ok={capabilities.mcpStdio} />
						<CapBadge label="MCP http" ok={capabilities.mcpHttp} />
						<CapBadge label="MCP sse" ok={capabilities.mcpSse} />
						<CapBadge
							label="Delegates terminal/*"
							ok={capabilities.terminalDelegation}
							empirical
						/>
					</div>
					<div className="mt-1.5 text-[10px] text-muted">
						Stdio is mandatory for all ACP agents, so circulo-mcp is always
						injected. terminal/* delegation is observed at runtime.
					</div>
				</div>
			) : null}

			{error ? (
				<div className="mb-4 rounded-lg border border-red-500/30 bg-red-500/10 px-3.5 py-2 text-[11px] text-red-200">
					{error}
				</div>
			) : null}

			<div className="space-y-3">
				{servers.map((server) => (
					<SettingRow
						key={server.id}
						label={
							<span className="flex items-center gap-2">
								{server.name}
								<span className="rounded bg-white/10 px-1.5 py-0.5 font-mono text-[10px] text-muted">
									{server.kind}
								</span>
								{server.builtIn ? (
									<span className="rounded bg-emerald-500/15 px-1.5 py-0.5 text-[10px] text-emerald-200">
										built-in
									</span>
								) : null}
							</span>
						}
						description={
							<code className="block truncate font-mono text-[10px] text-muted">
								{server.command} {server.args.join(" ")}
							</code>
						}
						control={
							<div className="flex items-center gap-4">
								<div className="flex flex-col items-center gap-0.5">
									<Switch
										checked={server.enabled}
										disabled={busy || server.builtIn}
										onCheckedChange={(next) =>
											void handleToggleState(server, next, server.autoLoad)
										}
										aria-label={`Enable ${server.name}`}
									/>
									<span className="text-[9px] uppercase tracking-wide text-muted">
										on-demand
									</span>
								</div>
								<div className="flex flex-col items-center gap-0.5">
									<Switch
										checked={server.autoLoad}
										disabled={busy || !server.enabled || server.builtIn}
										onCheckedChange={(next) =>
											void handleToggleState(server, server.enabled, next)
										}
										aria-label={`Auto-load ${server.name}`}
									/>
									<span className="text-[9px] uppercase tracking-wide text-muted">
										auto-load
									</span>
								</div>
								{!server.builtIn ? (
									<button
										type="button"
										onClick={() => void handleDelete(server.id)}
										className="rounded p-1 text-muted transition hover:bg-red-500/10 hover:text-red-300"
										aria-label={`Delete ${server.name}`}
									>
										<Trash2 className="size-3.5" />
									</button>
								) : null}
							</div>
						}
					/>
				))}

				{servers.length === 0 ? (
					<p className="rounded-lg border border-dashed border-border px-3.5 py-4 text-center text-[11px] text-muted">
						No servers registered yet. Add one with a preset, import from the
						project config, or use the guided form.
					</p>
				) : null}
			</div>

			{/* Presets */}
			<div className="mt-6">
				<div className="mb-2 text-[11px] font-medium uppercase tracking-wide text-muted">
					Presets
				</div>
				<div className="flex flex-wrap gap-2">
					{MCP_PRESETS.map((preset) => (
						<button
							key={preset.id}
							type="button"
							onClick={() => presetForm(preset)}
							className="rounded-md border border-border bg-black/20 px-2.5 py-1.5 text-[11px] text-fg transition hover:border-white/20 hover:bg-white/5"
							title={preset.hint}
						>
							{preset.label}
						</button>
					))}
				</div>
			</div>

			{/* Import from project config */}
			{imports.length > 0 ? (
				<div className="mt-6">
					<div className="mb-2 text-[11px] font-medium uppercase tracking-wide text-muted">
						Import from project config
					</div>
					<div className="space-y-2">
						{imports.map((candidate) => (
							<div
								key={candidate.id}
								className="flex items-center justify-between gap-3 rounded-lg border border-border bg-black/20 px-3.5 py-2.5"
							>
								<div className="min-w-0">
									<div className="flex items-center gap-2 text-sm text-fg">
										{candidate.name}
										<span className="rounded bg-white/10 px-1.5 py-0.5 font-mono text-[10px] text-muted">
											{candidate.kind}
										</span>
									</div>
									<div className="mt-0.5 truncate font-mono text-[10px] text-muted">
										{candidate.command} {candidate.args.join(" ")}
									</div>
									<div className="mt-0.5 text-[10px] text-muted">
										from {candidate.source}
									</div>
								</div>
								<button
									type="button"
									onClick={() => void handleImport(candidate)}
									disabled={busy}
									className="shrink-0 rounded-md border border-border px-2.5 py-1 text-[11px] text-fg transition hover:bg-white/5 disabled:opacity-50"
								>
									Import
								</button>
							</div>
						))}
					</div>
				</div>
			) : null}

			{/* Guided form */}
			<div className="mt-6">
				{!adding ? (
					<button
						type="button"
						onClick={() => {
							setAdding(true)
							setValidation({ state: "idle", tools: [], message: null })
						}}
						className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-[11px] text-fg transition hover:bg-white/5"
					>
						<Plus className="size-3.5" /> Add server
					</button>
				) : (
					<div className="rounded-lg border border-border bg-black/20 p-3.5">
						<div className="mb-3 flex items-center justify-between">
							<div className="text-sm font-medium text-fg">New server</div>
							<button
								type="button"
								onClick={() => {
									setAdding(false)
									setForm(EMPTY_FORM)
									setValidation({ state: "idle", tools: [], message: null })
								}}
								className="rounded p-1 text-muted transition hover:bg-white/5 hover:text-fg"
								aria-label="Cancel"
							>
								<X className="size-3.5" />
							</button>
						</div>

						<div className="space-y-3">
							<label className="block">
								<span className="mb-1 block text-[11px] text-muted">
									Name
								</span>
								<Input
									value={form.name}
									onChange={(e) =>
										setForm({ ...form, name: e.target.value })
									}
									placeholder="paper"
								/>
							</label>

							<div className="grid grid-cols-2 gap-3">
								<label className="block">
									<span className="mb-1 block text-[11px] text-muted">
										Transport
									</span>
									<Select
										value={form.kind}
										onValueChange={(kind) =>
											setForm({
												...form,
												kind: kind as McpServerKind,
											})
										}
									>
										<SelectTrigger className="w-full">
											<SelectValue />
										</SelectTrigger>
										<SelectContent>
											<SelectItem value="stdio">stdio</SelectItem>
											<SelectItem value="http">http</SelectItem>
											<SelectItem value="sse">sse</SelectItem>
										</SelectContent>
									</Select>
								</label>
								<label className="block">
									<span className="mb-1 block text-[11px] text-muted">
										{form.kind === "stdio"
											? "Command"
											: "URL"}
									</span>
									<Input
										value={form.command}
										onChange={(e) =>
											setForm({ ...form, command: e.target.value })
										}
										placeholder={
											form.kind === "stdio"
												? "npx -y @modelcontextprotocol/server-…"
												: "https://…"
										}
									/>
								</label>
							</div>

							{form.kind === "stdio" ? (
								<label className="block">
									<span className="mb-1 block text-[11px] text-muted">
										Arguments (space-separated)
									</span>
									<Input
										value={form.args}
										onChange={(e) =>
											setForm({ ...form, args: e.target.value })
										}
										placeholder="--port 3000"
									/>
								</label>
							) : null}

							<label className="block">
								<span className="mb-1 block text-[11px] text-muted">
									Environment variables (KEY=value, one per line)
								</span>
								<textarea
									value={form.env}
									onChange={(e) =>
										setForm({ ...form, env: e.target.value })
									}
									rows={3}
									className="w-full resize-none rounded-md border border-border bg-black/30 px-2.5 py-1.5 font-mono text-[11px] text-fg placeholder:text-muted focus-visible:border-white/20 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-white/10"
									placeholder={"API_KEY=…"}
								/>
							</label>

							<SettingRow
								label="Auto-load"
								description="Inject this server natively into every session with its full tool catalogue."
								control={
									<Switch
										checked={form.autoLoad}
										onCheckedChange={(next) =>
											setForm({ ...form, autoLoad: next })
										}
									/>
								}
							/>

							{validation.state !== "idle" ? (
								<div
									className={`rounded-md border px-3 py-2 text-[11px] ${
										validation.state === "ok"
											? "border-emerald-500/25 bg-emerald-500/5 text-emerald-200/90"
											: validation.state === "failed"
												? "border-red-500/30 bg-red-500/10 text-red-200"
												: "border-border bg-white/5 text-muted"
									}`}
								>
									{validation.state === "testing"
										? "Launching server and listing tools…"
										: validation.state === "ok"
											? `Connected — ${validation.tools.length} tools available`
											: validation.message}
								</div>
							) : null}

							<div className="flex gap-2">
								<button
									type="button"
									onClick={() => void handleValidate()}
									disabled={busy || !form.command || !form.name}
									className="flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-[11px] text-fg transition hover:bg-white/5 disabled:opacity-50"
								>
									{validation.state === "testing" ? (
										<Loader2 className="size-3 animate-spin" />
									) : (
										<RefreshCw className="size-3" />
									)}
									Test server
								</button>
								<button
									type="button"
									onClick={() => void handleSave()}
									disabled={busy || !form.command || !form.name}
									className="rounded-md bg-white/10 px-3 py-1.5 text-[11px] font-medium text-fg transition hover:bg-white/15 disabled:opacity-50"
								>
									Save
								</button>
							</div>
						</div>
					</div>
				)}
			</div>
		</div>
	)
}

function buildServer(form: typeof EMPTY_FORM): ManagedMcpServer | null {
	const name = form.name.trim()
	if (!name) return null
	const env = form.env
		.split("\n")
		.map((line) => line.trim())
		.filter(Boolean)
		.map((line) => {
			const eq = line.indexOf("=")
			if (eq === -1) return { name: line, value: "" }
			return { name: line.slice(0, eq).trim(), value: line.slice(eq + 1).trim() }
		})
	return {
		id: slugify(name),
		name,
		kind: form.kind,
		command: form.command.trim(),
		args: form.args
			.split(/\s+/)
			.map((a) => a.trim())
			.filter(Boolean),
		env,
		enabled: true,
		autoLoad: form.autoLoad,
		builtIn: false,
	}
}
