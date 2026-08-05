export interface AgentDescriptor {
	id: string
	label: string
}

/** Frontend mirror of Rust agent registry — extend when adding agents. */
export const KNOWN_AGENTS: Record<string, AgentDescriptor> = {
	opencode: { id: "opencode", label: "OpenCode" },
	"cursor-agent": { id: "cursor-agent", label: "Cursor Agent" },
	grok: { id: "grok", label: "Grok" },
	pi: { id: "pi", label: "Pi" },
	custom: { id: "custom", label: "Custom ACP" },
}

export function agentLabel(agentId: string | null | undefined): string {
	if (!agentId) return KNOWN_AGENTS.opencode.label
	return KNOWN_AGENTS[agentId]?.label ?? agentId
}

export type AgentConnectionStatus = "loading" | "ready" | "disconnected"

export interface AgentRuntimeState {
	id: string
	label: string
	command: string
	status: AgentConnectionStatus
	detail: string | null
}

export function aggregateAgentStatus(
	agents: AgentRuntimeState[],
): AgentConnectionStatus {
	if (agents.some((agent) => agent.status === "loading")) return "loading"
	if (agents.length > 0 && agents.every((agent) => agent.status === "ready")) {
		return "ready"
	}
	if (agents.some((agent) => agent.status === "ready")) return "loading"
	return "disconnected"
}

export function statusLabel(status: AgentConnectionStatus): string {
	switch (status) {
		case "ready":
			return "Ready"
		case "loading":
			return "Connecting"
		case "disconnected":
			return "Disconnected"
	}
}

export function resolveAgentConnectionStatus({
	connected,
	statusConnecting,
	progress,
	available,
}: {
	connected: boolean
	statusConnecting: boolean
	progress: string | null
	available: boolean
}): AgentConnectionStatus {
	if (!available) return "disconnected"
	if (statusConnecting) return "loading"
	if (connected) return "ready"
	if (progress) return "loading"
	return "disconnected"
}

export function resolveAgentDetail({
	connected,
	statusConnecting,
	progress,
	available,
	installHint,
	sessionStatus,
}: {
	connected: boolean
	statusConnecting: boolean
	progress: string | null
	available: boolean
	installHint: string | null
	sessionStatus: string
}): string | null {
	if (!available) return installHint
	if (statusConnecting) return progress || "Opening session…"
	if (connected) return "Connected via ACP"
	if (progress) return progress
	if (sessionStatus === "disconnected") return "Agent disconnected"
	return null
}
