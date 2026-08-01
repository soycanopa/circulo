import { Monitor } from "lucide-react"
import {
	aggregateAgentStatus,
	type AgentConnectionStatus,
	type AgentRuntimeState,
	statusLabel,
} from "@/lib/agent-registry"
import { cn } from "@/lib/utils"

interface AgentStatusIndicatorProps {
	agents: AgentRuntimeState[]
}

const dotClass: Record<AgentConnectionStatus, string> = {
	ready: "bg-emerald-400",
	loading: "animate-pulse bg-amber-400",
	disconnected: "bg-red-400/90",
}

const rowDotClass: Record<AgentConnectionStatus, string> = {
	ready: "bg-emerald-400",
	loading: "animate-pulse bg-amber-400",
	disconnected: "bg-red-400/90",
}

function AgentTooltipRow({ agent }: { agent: AgentRuntimeState }) {
	return (
		<div className="border-b border-border py-2 last:border-b-0 last:pb-0 first:pt-0">
			<div className="flex items-center justify-between gap-2">
				<p className="text-xs font-medium text-fg">{agent.label}</p>
				<span className="inline-flex items-center gap-1.5 text-[10px] text-muted">
					<span
						className={cn("size-1.5 rounded-full", rowDotClass[agent.status])}
						aria-hidden
					/>
					{statusLabel(agent.status)}
				</span>
			</div>
			<p className="mt-1 font-mono text-[10px] text-muted">{agent.command}</p>
			{agent.detail ? (
				<p className="mt-1 text-[10px] leading-snug text-muted/90">
					{agent.detail}
				</p>
			) : null}
		</div>
	)
}

export function AgentStatusIndicator({ agents }: AgentStatusIndicatorProps) {
	const status = aggregateAgentStatus(agents)
	const primary = agents[0]

	return (
		<div className="group relative min-w-0">
			<button
				type="button"
				className="flex min-w-0 items-center gap-2 rounded-md py-0.5 text-left outline-none focus-visible:ring-1 focus-visible:ring-white/20"
				aria-label={`ACP agent status: ${statusLabel(status)}`}
			>
				<span className="relative shrink-0">
					<Monitor className="size-4 text-muted" aria-hidden />
					<span
						className={cn(
							"absolute -bottom-px -right-px size-2 rounded-full border border-frame",
							dotClass[status],
						)}
						aria-hidden
					/>
				</span>
				<span className="text-[11px] text-muted">ACP</span>
			</button>

			<div
				role="tooltip"
				className={cn(
					"pointer-events-none absolute bottom-full left-0 z-50 mb-2 w-56",
					"rounded-md border border-border bg-surface px-3 py-2 shadow-lg",
					"opacity-0 transition-opacity duration-150",
					"group-hover:opacity-100 group-focus-within:opacity-100",
				)}
			>
				<p className="mb-2 text-[10px] font-medium uppercase tracking-wider text-muted/80">
					ACP agents
				</p>
				{agents.length > 0 ? (
					agents.map((agent) => <AgentTooltipRow key={agent.id} agent={agent} />)
				) : (
					<p className="text-xs text-muted">No agents configured</p>
				)}
				{primary ? (
					<p className="mt-2 border-t border-border pt-2 text-[10px] text-muted/80">
						Active: {primary.label}
					</p>
				) : null}
			</div>
		</div>
	)
}
