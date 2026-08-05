import { useEffect, useState } from "react"
import { Switch } from "@/components/ui/switch"
import { SectionHeader, SettingRow } from "@/components/settings/sections/section-ui"
import { agentLabel } from "@/lib/agent-registry"
import { listAgents, setEnabledAgents } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import type { AgentDescriptor, AppSettings } from "@/types/acp"

interface AgentsSectionProps {
	agentCommand: string
	preferredAgentId?: string | null
	enabledAgentIds?: string[]
	onPreferredAgentChange?: (agentId: string) => void
	onEnabledAgentsChange?: (settings: AppSettings) => void
}

export function AgentsSection({
	agentCommand,
	preferredAgentId,
	enabledAgentIds = ["opencode", "cursor-agent"],
	onPreferredAgentChange,
	onEnabledAgentsChange,
}: AgentsSectionProps) {
	const [agents, setAgents] = useState<AgentDescriptor[]>([])
	const [enabledIds, setEnabledIds] = useState(enabledAgentIds)
	const [selectedAgentId, setSelectedAgentId] = useState(
		preferredAgentId ?? "opencode",
	)
	const [saving, setSaving] = useState(false)

	useEffect(() => {
		void listAgents().then(setAgents)
	}, [])

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
			if (preferredChanged) onPreferredAgentChange?.(nextPreferred)
		} finally {
			setSaving(false)
		}
	}

	async function handleDefaultAgentChange(agentId: string) {
		setSelectedAgentId(agentId)
		setSaving(true)
		try {
			await onPreferredAgentChange?.(agentId)
		} finally {
			setSaving(false)
		}
	}

	const activeAgent = agents.find((a) => a.id === selectedAgentId)

	return (
		<div>
			<SectionHeader
				title="Agents"
				description="Enable agents to show them in the chat composer and pick the default."
			/>
			<div className="space-y-3">
				<SettingRow
					label="Default agent"
					description="Used when opening a project or starting a new chat. Switching respawns the ACP process."
					control={
						selectableAgents.length > 0 ? (
							<select
								value={selectedAgentId}
								disabled={saving}
								onChange={(event) =>
									void handleDefaultAgentChange(event.target.value)
								}
								className="rounded-md border border-border bg-black/20 px-2 py-1.5 text-sm text-fg"
							>
								{selectableAgents.map((agent) => (
									<option key={agent.id} value={agent.id}>
										{agentLabel(agent.id)}
									</option>
								))}
							</select>
						) : (
							<span className="text-xs text-muted">
								Enable at least one available agent.
							</span>
						)
					}
				/>

				{activeAgent ? (
					<div className="rounded-lg border border-border bg-black/20 px-3.5 py-3">
						<div className="text-sm text-fg">{agentLabel(activeAgent.id)}</div>
						<div className="mt-1 font-mono text-[11px] text-muted">
							{activeAgent.command}
						</div>
					</div>
				) : null}

				<div className="pt-2">
					<div className="text-[11px] uppercase tracking-wider text-muted">
						Enabled agents
					</div>
					<div className="mt-2 space-y-2">
						{agents.map((agent) => {
							const isEnabled = enabledSet.has(agent.id)
							return (
								<SettingRow
									key={agent.id}
									label={agentLabel(agent.id)}
									description={
										<span className="flex items-center gap-2">
											<span
												className={cn(
													agent.available
														? "text-emerald-400/80"
														: "text-muted",
												)}
											>
												{agent.available ? "Available" : "Unavailable"}
											</span>
											<span className="font-mono">{agent.command}</span>
										</span>
									}
									control={
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
										/>
									}
								/>
							)
						})}
					</div>
				</div>

				<p className="text-[11px] text-muted">
					Custom: set{" "}
					<code className="rounded bg-white/5 px-1">CIRCULO_CUSTOM_ACP</code>{" "}
					(program + args) to override the default command
					<span className="font-mono"> {agentCommand}</span>.
				</p>
			</div>
		</div>
	)
}
