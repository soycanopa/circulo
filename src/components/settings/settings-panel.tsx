import { X } from "lucide-react"
import { getDefaultChatsPath, listAgents, setPreferredAgent } from "@/lib/tauri"
import { useEffect, useState } from "react"
import type { AgentDescriptor } from "@/types/acp"

interface SettingsPanelProps {
	open: boolean
	onClose: () => void
	agentCommand: string
	preferredAgentId?: string | null
	onPreferredAgentChange?: (agentId: string) => void
}

export function SettingsPanel({
	open,
	onClose,
	agentCommand,
	preferredAgentId,
	onPreferredAgentChange,
}: SettingsPanelProps) {
	const [chatsPath, setChatsPath] = useState("—")
	const [agents, setAgents] = useState<AgentDescriptor[]>([])
	const [selectedAgentId, setSelectedAgentId] = useState(
		preferredAgentId ?? "opencode",
	)
	const [saving, setSaving] = useState(false)

	useEffect(() => {
		if (!open) return
		void getDefaultChatsPath().then(setChatsPath)
		void listAgents().then(setAgents)
	}, [open])

	useEffect(() => {
		setSelectedAgentId(preferredAgentId ?? "opencode")
	}, [preferredAgentId])

	async function handleAgentChange(agentId: string) {
		setSelectedAgentId(agentId)
		setSaving(true)
		try {
			await setPreferredAgent(agentId)
			onPreferredAgentChange?.(agentId)
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
				<div className="space-y-4 px-4 py-4 text-xs">
					<div>
						<div className="text-[11px] uppercase tracking-wider text-muted">
							Agent
						</div>
						{agents.length > 0 ? (
							<select
								value={selectedAgentId}
								disabled={saving}
								onChange={(event) => void handleAgentChange(event.target.value)}
								className="mt-1 w-full rounded-md border border-border bg-black/20 px-2 py-1.5 text-sm text-fg"
							>
								{agents.map((agent) => (
									<option
										key={agent.id}
										value={agent.id}
										disabled={!agent.available}
									>
										{agent.label}
										{agent.available ? "" : " (unavailable)"}
									</option>
								))}
							</select>
						) : (
							<p className="mt-1 font-mono text-fg">{agentCommand}</p>
						)}
						<p className="mt-1 font-mono text-[11px] text-muted">
							{activeAgent?.command ?? agentCommand}
						</p>
						<p className="mt-1 text-[11px] text-muted">
							Switching agents applies on the next project open.
						</p>
						<p className="mt-1 text-[11px] text-muted">
							Custom: set{" "}
							<code className="rounded bg-white/5 px-1">CIRCULO_CUSTOM_ACP</code>{" "}
							(program + args).
						</p>
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
							Circulo v0.3.0 — desktop ACP client
						</p>
					</div>
				</div>
			</div>
		</div>
	)
}
