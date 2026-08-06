import { getDefaultStore } from "jotai"
import { listAgents } from "@/lib/tauri"
import { agentsAtom } from "@/stores/atoms"
import type { AgentDescriptor } from "@/types/acp"

/** Load agents from Rust (cached server-side) and mirror into Jotai. */
export async function refreshAgentsList(
	store = getDefaultStore(),
): Promise<AgentDescriptor[]> {
	const agents = await listAgents()
	store.set(agentsAtom, agents)
	return agents
}
