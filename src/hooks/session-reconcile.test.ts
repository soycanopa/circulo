import { createStore } from "jotai"
import { describe, expect, it } from "vitest"
import {
	reconcileSessionFromProjectStatus,
	removeSessionFromUi,
} from "@/hooks/session-reconcile"
import {
	activeSessionIdAtom,
	agentConnectedAtom,
	configOptionsAtom,
	connectionGenerationAtom,
	historyMessagesAtom,
	historyViewSessionIdAtom,
	sessionsAtom,
} from "@/stores/atoms"
import type { ConfigOption, ProjectStatus } from "@/types/acp"

const modelOption: ConfigOption = {
	id: "model",
	name: "Model",
	category: "model",
	currentValue: "opencode/gpt-5",
	options: [
		{ value: "opencode/gpt-5", name: "GPT-5" },
	],
}

const modeOption: ConfigOption = {
	id: "mode",
	name: "Mode",
	category: "mode",
	currentValue: "agent",
	options: [{ value: "agent", name: "Agent" }],
}

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
		mcpStdio: true,
		mcpHttp: false,
		mcpSse: false,
		terminalDelegation: false,
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

	it("writes config options into the session slot (not only the global atom)", () => {
		const store = createStore()
		reconcileSessionFromProjectStatus(store, {
			...baseStatus,
			configOptions: [modelOption, modeOption],
		})

		expect(store.get(configOptionsAtom)).toEqual([modelOption, modeOption])
		expect(store.get(sessionsAtom)["session-abc"]?.configOptions).toEqual([
			modelOption,
			modeOption,
		])
	})
})

describe("removeSessionFromUi", () => {
	it("clears live session, history view, and session slot", () => {
		const store = createStore()
		store.set(activeSessionIdAtom, "session-a")
		store.set(historyViewSessionIdAtom, "session-a")
		store.set(historyMessagesAtom, [
			{
				id: "h1",
				role: "user",
				content: "hello",
				toolCalls: [],
				timestamp: 1,
			},
		])
		store.set(sessionsAtom, {
			"session-a": {
				messages: [
					{
						id: "m1",
						role: "assistant",
						content: "hi",
						toolCalls: [],
						timestamp: 2,
					},
				],
				streaming: "",
				promptInFlight: false,
				status: "idle",
				configOptions: [],
				contextUsage: null,
			},
			"session-b": {
				messages: [],
				streaming: "",
				promptInFlight: false,
				status: "idle",
				configOptions: [],
				contextUsage: null,
			},
		})

		removeSessionFromUi(store, "session-a")

		expect(store.get(activeSessionIdAtom)).toBeNull()
		expect(store.get(historyViewSessionIdAtom)).toBeNull()
		expect(store.get(historyMessagesAtom)).toEqual([])
		expect(store.get(sessionsAtom)["session-a"]).toBeUndefined()
		expect(store.get(sessionsAtom)["session-b"]).toBeDefined()
	})

	it("leaves unrelated sessions untouched", () => {
		const store = createStore()
		store.set(activeSessionIdAtom, "session-b")
		store.set(sessionsAtom, {
			"session-a": {
				messages: [
					{
						id: "m1",
						role: "user",
						content: "old",
						toolCalls: [],
						timestamp: 1,
					},
				],
				streaming: "",
				promptInFlight: false,
				status: "idle",
				configOptions: [],
				contextUsage: null,
			},
			"session-b": {
				messages: [],
				streaming: "",
				promptInFlight: false,
				status: "idle",
				configOptions: [],
				contextUsage: null,
			},
		})

		removeSessionFromUi(store, "session-a")

		expect(store.get(activeSessionIdAtom)).toBe("session-b")
		expect(store.get(sessionsAtom)["session-a"]).toBeUndefined()
		expect(store.get(sessionsAtom)["session-b"]).toBeDefined()
	})
})
