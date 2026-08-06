import { useAtomValue } from "jotai"
import { useEffect, useMemo } from "react"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { refreshAgentsList } from "@/lib/agents-cache"
import { agentLabel } from "@/lib/agent-registry"
import { agentsAtom, visiblePromptInFlightAtom } from "@/stores/atoms"
import type { AgentDescriptor } from "@/types/acp"

interface AgentSelectorProps {
	enabledAgentIds: string[]
	preferredAgentId: string | null | undefined
	onAgentChange: (agentId: string) => void | Promise<void>
}

function mergeEnabledAgents(
	enabledAgentIds: string[],
	agents: AgentDescriptor[],
): AgentDescriptor[] {
	const byId = new Map(agents.map((agent) => [agent.id, agent]))
	return enabledAgentIds.map(
		(id) =>
			byId.get(id) ?? {
				id,
				label: agentLabel(id),
				command: "",
				available: false,
			},
	)
}

export function AgentSelector({
	enabledAgentIds,
	preferredAgentId,
	onAgentChange,
}: AgentSelectorProps) {
	const promptInFlight = useAtomValue(visiblePromptInFlightAtom)
	const agents = useAtomValue(agentsAtom)
	const enabledKey = enabledAgentIds.join("\0")

	useEffect(() => {
		if (agents.length > 0) return
		void refreshAgentsList().catch(() => {
			// Composer falls back to enabled ids without availability hints.
		})
	}, [agents.length, enabledKey])

	const entries = useMemo(
		() => mergeEnabledAgents(enabledAgentIds, agents),
		[enabledAgentIds, agents],
	)

	if (entries.length === 0) return null

	const availableEntries = entries.filter((agent) => agent.available)
	const currentId =
		preferredAgentId && entries.some((agent) => agent.id === preferredAgentId)
			? preferredAgentId
			: (availableEntries[0]?.id ?? entries[0]?.id)

	if (!currentId) return null

	return (
		<div className="shrink-0">
			<Select
				disabled={promptInFlight}
				value={currentId}
				onValueChange={(next) => {
					const hit = entries.find((agent) => agent.id === next)
					if (hit?.available) void onAgentChange(next)
				}}
			>
				<SelectTrigger
					aria-label="Agent"
					className="h-auto w-auto min-w-0 max-w-[10rem] gap-0.5 rounded-md border-0 bg-transparent px-1.5 py-1 text-[13px] text-white/70 shadow-none transition-colors hover:bg-white/[0.08] hover:text-white/90 focus-visible:border-0 focus-visible:bg-white/[0.08] focus-visible:ring-0 data-[state=open]:bg-white/[0.08] data-[placeholder]:text-white/50 [&>svg]:size-3.5 [&>svg]:text-white/55"
				>
					<SelectValue placeholder="Agent">
						{agentLabel(currentId)}
					</SelectValue>
				</SelectTrigger>
				<SelectContent position="popper" align="start">
					{entries.map((agent) => (
						<SelectItem
							key={agent.id}
							value={agent.id}
							disabled={!agent.available}
						>
							{agentLabel(agent.id)}
							{!agent.available ? " (unavailable)" : ""}
						</SelectItem>
					))}
				</SelectContent>
			</Select>
		</div>
	)
}
