import { getDefaultStore } from "jotai"
import { agentConnectedAtom } from "@/stores/atoms"

export const AGENT_READY_EVENT = "circulo:agent-ready"

export function waitForAgentReady(timeoutMs = 45_000): Promise<void> {
	if (getDefaultStore().get(agentConnectedAtom)) {
		return Promise.resolve()
	}

	return new Promise((resolve, reject) => {
		const timer = window.setTimeout(() => {
			window.removeEventListener(AGENT_READY_EVENT, onReady)
			reject(new Error("Tiempo de espera agotado al conectar el agente"))
		}, timeoutMs)

		function onReady() {
			window.clearTimeout(timer)
			window.removeEventListener(AGENT_READY_EVENT, onReady)
			resolve()
		}

		window.addEventListener(AGENT_READY_EVENT, onReady)
	})
}