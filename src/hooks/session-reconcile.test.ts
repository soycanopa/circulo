import { createStore } from "jotai"
import { describe, expect, it } from "vitest"
import { reconcileSessionFromProjectStatus } from "@/hooks/session-reconcile"
import {
	activeSessionIdAtom,
	agentConnectedAtom,
	connectionGenerationAtom,
	historyMessagesAtom,
	historyViewSessionIdAtom,
	sessionsAtom,
} from "@/stores/atoms"
import type { ProjectStatus } from "@/types/acp"

const baseStatus: ProjectStatus = {
	connected: true,
	projectPath: "/tmp/project",
	agentId: "opencode",
	connectionGeneration: 2,
	sessionId: "session-abc",
	configOptions: [],
	capabilities: {
		loadSession: true,
		listSessions: false,
		resumeSession: false,
		closeSession: true,
		concurrentSessions: true,
	},
	agentCommand: "opencode acp",
}

describe("reconcileSessionFromProjectStatus", () => {
	it("binds the active session from command status", () => {
		const store = createStore()
		store.set(historyViewSessionIdAtom, "saved-1")
		store.set(historyMessagesAtom, [
			{
				id: "h1",
				role: "user",
				content: "hello",
				toolCalls: [],
				timestamp: 1,
			},
		])

		reconcileSessionFromProjectStatus(store, baseStatus)

		expect(store.get(connectionGenerationAtom)).toBe(2)
		expect(store.get(agentConnectedAtom)).toBe(true)
		expect(store.get(activeSessionIdAtom)).toBe("session-abc")
		expect(store.get(historyViewSessionIdAtom)).toBeNull()
		expect(store.get(historyMessagesAtom)).toEqual([])
		expect(store.get(sessionsAtom)["session-abc"]).toBeDefined()
	})
})
