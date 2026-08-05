import { Trash2, X } from "lucide-react"
import {
	getDefaultChatsPath,
	listAgents,
	saveAutomation,
	setEnabledAgents,
} from "@/lib/tauri"
import { agentLabel } from "@/lib/agent-registry"
import { useEffect, useState } from "react"
import { Switch } from "@/components/ui/switch"
import type { AgentDescriptor, AppSettings, Automation } from "@/types/acp"

interface SettingsPanelProps {
	open: boolean
	onClose: () => void
	agentCommand: string
	preferredAgentId?: string | null
	enabledAgentIds?: string[]
	allowedToolPatterns?: string[]
	onPreferredAgentChange?: (agentId: string) => void
	onEnabledAgentsChange?: (settings: AppSettings) => void
	onSetAllowedTool?: (pattern: string, enabled: boolean) => Promise<void>
	automations: Automation[]
	onAutomationsChange: () => void
	onDeleteAutomation: (id: string) => Promise<void>
}

export function SettingsPanel({
	open,
	onClose,
	agentCommand,
	preferredAgentId,
	enabledAgentIds = ["opencode", "cursor-agent"],
	allowedToolPatterns = [],
	onPreferredAgentChange,
	onEnabledAgentsChange,
	onSetAllowedTool,
	automations,
	onAutomationsChange,
	onDeleteAutomation,
}: SettingsPanelProps) {
	const [chatsPath, setChatsPath] = useState("—")
	const [agents, setAgents] = useState<AgentDescriptor[]>([])
	const [enabledIds, setEnabledIds] = useState(enabledAgentIds)
	const [selectedAgentId, setSelectedAgentId] = useState(
		preferredAgentId ?? "opencode",
	)
	const [title, setTitle] = useState("")
	const [prompt, setPrompt] = useState("")
	const [saving, setSaving] = useState(false)
	const [newPattern, setNewPattern] = useState("")

	useEffect(() => {
		if (!open) return
		void getDefaultChatsPath().then(setChatsPath)
		void listAgents().then(setAgents)
	}, [open])

	useEffect(() => {
		setEnabledIds(enabledAgentIds)
	}, [enabledAgentIds])

	useEffect(() => {
		setSelectedAgentId(preferredAgentId ?? "opencode")
	}, [preferredAgentId])

	const enabledSet = new Set(enabledIds)
	const selectableAgents = agents.filter(
		(agent) => enabledSet.has(agent.id) && agent.available,
	)

	async function handleDefaultAgentChange(agentId: string) {
		setSelectedAgentId(agentId)
		setSaving(true)
		try {
			await onPreferredAgentChange?.(agentId)
		} finally {
			setSaving(false)
		}
	}

	async function handleToggleAgent(agentId: string, nextEnabled: boolean) {
		const nextIds = nextEnabled
			? [...enabledIds, agentId]
			: enabledIds.filter((id) => id !== agentId)

		if (nextIds.length === 0) return

		setSaving(true)
		try {
			const settings = await setEnabledAgents(nextIds)
			setEnabledIds(settings.enabledAgentIds ?? nextIds)
			const nextPreferred = settings.preferredAgentId ?? selectedAgentId
			const preferredChanged = nextPreferred !== selectedAgentId
			setSelectedAgentId(nextPreferred)
			onEnabledAgentsChange?.(settings)
			if (preferredChanged) {
				onPreferredAgentChange?.(nextPreferred)
			}
		} finally {
			setSaving(false)
		}
	}

	async function handleSaveAutomation() {
		if (!title.trim() || !prompt.trim()) return
		setSaving(true)
		try {
			await saveAutomation(title.trim(), prompt.trim())
			setTitle("")
			setPrompt("")
			onAutomationsChange()
		} finally {
			setSaving(false)
		}
	}

	async function handleAddAllowedTool() {
		const pattern = newPattern.trim()
		if (!pattern) return
		setSaving(true)
		try {
			await onSetAllowedTool?.(pattern, true)
			setNewPattern("")
		} finally {
			setSaving(false)
		}
	}

	if (!open) return null

	const activeAgent = agents.find((a) => a.id === selectedAgentId)

	return (
		<div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
			<div
				role="dialog"
				aria-modal="true"
				className="frosted-strong w-full max-w-md rounded-lg border border-border shadow-xl"
			>
				<div className="flex items-center justify-between border-b border-border px-4 py-3">
					<h2 className="text-sm font-medium text-fg">Settings</h2>
					<button
						type="button"
						onClick={onClose}
						className="rounded p-1 text-muted transition hover:bg-white/5 hover:text-fg"
					>
						<X className="size-4" />
					</button>
				</div>
				<div className="max-h-[70vh] space-y-4 overflow-y-auto px-4 py-4 text-xs">
					<div>
						<div className="text-[11px] uppercase tracking-wider text-muted">
							ACP agents
						</div>
						<p className="mt-1 text-[11px] text-muted">
							Enable agents to show them in the chat composer.
						</p>
						{agents.length > 0 ? (
							<ul className="mt-2 space-y-2">
								{agents.map((agent) => {
									const isEnabled = enabledSet.has(agent.id)
									return (
										<li
											key={agent.id}
											className="flex items-start justify-between gap-3 rounded border border-border bg-black/20 px-2.5 py-2"
										>
											<div className="min-w-0 flex-1">
												<div className="flex items-center gap-2">
													<span className="text-sm text-fg">
														{agentLabel(agent.id)}
													</span>
													<span
														className={
															agent.available
																? "text-[10px] text-emerald-400/80"
																: "text-[10px] text-muted"
														}
													>
														{agent.available ? "Available" : "Unavailable"}
													</span>
												</div>
												<p className="mt-0.5 font-mono text-[10px] text-muted">
													{agent.command}
												</p>
											</div>
											<Switch
												checked={isEnabled}
												disabled={
													saving ||
													!agent.available ||
													(isEnabled && enabledIds.length <= 1)
												}
												onCheckedChange={(next) =>
													void handleToggleAgent(agent.id, next)
												}
												aria-label={`Enable ${agentLabel(agent.id)}`}
												className="mt-0.5"
											/>
										</li>
									)
								})}
							</ul>
						) : null}
					</div>

					<div>
						<div className="text-[11px] uppercase tracking-wider text-muted">
							Default agent
						</div>
						<p className="mt-1 text-[11px] text-muted">
							Used when opening a project or starting a new chat. Switching
							respawns the ACP process.
						</p>
						{selectableAgents.length > 0 ? (
							<select
								value={selectedAgentId}
								disabled={saving}
								onChange={(event) =>
									void handleDefaultAgentChange(event.target.value)
								}
								className="mt-2 w-full rounded-md border border-border bg-black/20 px-2 py-1.5 text-sm text-fg"
							>
								{selectableAgents.map((agent) => (
									<option key={agent.id} value={agent.id}>
										{agentLabel(agent.id)}
									</option>
								))}
							</select>
						) : (
							<p className="mt-2 text-muted">
								Enable at least one available agent.
							</p>
						)}
						<p className="mt-2 font-mono text-[11px] text-muted">
							{activeAgent?.command ?? agentCommand}
						</p>
						<p className="mt-1 text-[11px] text-muted">
							Custom: set{" "}
							<code className="rounded bg-white/5 px-1">CIRCULO_CUSTOM_ACP</code>{" "}
							(program + args).
						</p>
					</div>

					<div>
						<div className="text-[11px] uppercase tracking-wider text-muted">
							Automations
						</div>
						<p className="mt-1 text-[11px] text-muted">
							Saved prompts appear in the command palette (⌘K).
						</p>
						{automations.length > 0 ? (
							<ul className="mt-2 space-y-1">
								{automations.map((item) => (
									<li
										key={item.id}
										className="flex items-center justify-between gap-2 rounded border border-border bg-black/20 px-2 py-1.5"
									>
										<span className="truncate text-fg">{item.title}</span>
										<button
											type="button"
											onClick={() => void onDeleteAutomation(item.id)}
											className="shrink-0 rounded p-1 text-muted hover:bg-white/5 hover:text-red-300"
											title="Delete automation"
										>
											<Trash2 className="size-3.5" />
										</button>
									</li>
								))}
							</ul>
						) : (
							<p className="mt-2 text-muted">No automations yet.</p>
						)}
						<div className="mt-3 space-y-2">
							<input
								value={title}
								onChange={(event) => setTitle(event.target.value)}
								placeholder="Title"
								className="w-full rounded-md border border-border bg-black/20 px-2 py-1.5 text-sm text-fg"
							/>
							<textarea
								value={prompt}
								onChange={(event) => setPrompt(event.target.value)}
								placeholder="Prompt to send"
								rows={3}
								className="w-full resize-none rounded-md border border-border bg-black/20 px-2 py-1.5 text-sm text-fg"
							/>
							<button
								type="button"
								disabled={saving || !title.trim() || !prompt.trim()}
								onClick={() => void handleSaveAutomation()}
								className="rounded-md border border-border px-2 py-1 text-[11px] text-fg transition hover:bg-white/5 disabled:opacity-40"
							>
								Save automation
							</button>
						</div>
					</div>

				<div>
					<div className="text-[11px] uppercase tracking-wider text-muted">
						Always allow tools
					</div>
					<p className="mt-1 text-[11px] text-muted">
						Tools matching these patterns (exact or <code className="rounded bg-white/5 px-1">*</code> glob)
						skip the permission prompt. Use the bookmark button on a permission card to add one.
					</p>
					{allowedToolPatterns.length > 0 ? (
						<ul className="mt-2 space-y-1">
							{allowedToolPatterns.map((pattern) => (
								<li
									key={pattern}
									className="flex items-center justify-between gap-2 rounded border border-border bg-black/20 px-2 py-1.5"
								>
									<span className="truncate font-mono text-fg/90">
										{pattern}
									</span>
									<button
										type="button"
										disabled={saving}
										onClick={() => void onSetAllowedTool?.(pattern, false)}
										className="shrink-0 rounded p-1 text-muted hover:bg-white/5 hover:text-red-300 disabled:opacity-40"
										title="Forget pattern"
									>
										<Trash2 className="size-3.5" />
									</button>
								</li>
							))}
						</ul>
					) : (
						<p className="mt-2 text-muted">No remembered tools yet.</p>
					)}
					<div className="mt-2 flex gap-1.5">
						<input
							value={newPattern}
							onChange={(event) => setNewPattern(event.target.value)}
							onKeyDown={(event) => {
								if (event.key === "Enter") void handleAddAllowedTool()
							}}
							placeholder="e.g. bash or edit*"
							className="w-full rounded-md border border-border bg-black/20 px-2 py-1.5 text-sm text-fg"
						/>
						<button
							type="button"
							disabled={saving || !newPattern.trim() || !onSetAllowedTool}
							onClick={() => void handleAddAllowedTool()}
							className="shrink-0 rounded-md border border-border px-2 py-1 text-[11px] text-fg transition hover:bg-white/5 disabled:opacity-40"
						>
							Add
						</button>
					</div>
				</div>

				<div>
					<div className="text-[11px] uppercase tracking-wider text-muted">
						General chats folder
					</div>
					<p className="mt-1 break-all font-mono text-fg/90">{chatsPath}</p>
				</div>
					<div>
						<div className="text-[11px] uppercase tracking-wider text-muted">
							About
						</div>
						<p className="mt-1 text-muted">
							Circulo v0.4.0 — desktop ACP client
						</p>
					</div>
				</div>
			</div>
		</div>
	)
}
