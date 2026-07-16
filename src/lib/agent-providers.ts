export type AgentProviderId = "opencode" | "grok" | "cline" | "codex"

export interface AgentProviderDefinition {
	id: AgentProviderId
	label: string
	shortLabel: string
	command: string
	description: string
	/** Circulo can spawn this agent over ACP today. */
	acpReady: boolean
	probeProgram: string
}

export const AGENT_PROVIDERS: AgentProviderDefinition[] = [
	{
		id: "opencode",
		label: "OpenCode",
		shortLabel: "OpenCode",
		command: "opencode acp",
		description: "Agente principal vía ACP stdio.",
		acpReady: true,
		probeProgram: "opencode",
	},
	{
		id: "grok",
		label: "Grok Build",
		shortLabel: "Grok",
		command: "grok agent stdio",
		description: "CLI de Grok Build. ACP en Circulo próximamente.",
		acpReady: false,
		probeProgram: "grok",
	},
	{
		id: "cline",
		label: "Cline",
		shortLabel: "Cline",
		command: "cline acp",
		description: "Agente Cline vía ACP.",
		acpReady: false,
		probeProgram: "cline",
	},
	{
		id: "codex",
		label: "Codex",
		shortLabel: "Codex",
		command: "codex acp",
		description: "OpenAI Codex CLI vía ACP.",
		acpReady: false,
		probeProgram: "codex",
	},
]

export function getAgentProvider(id: string): AgentProviderDefinition | undefined {
	return AGENT_PROVIDERS.find((entry) => entry.id === id)
}

export function isAgentProviderId(value: string): value is AgentProviderId {
	return AGENT_PROVIDERS.some((entry) => entry.id === value)
}