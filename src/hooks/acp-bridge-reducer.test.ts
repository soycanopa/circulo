import { createStore } from "jotai"
import { beforeEach, describe, expect, it, vi } from "vitest"
import {
	processAcpEvent,
	type AcpBridgeRefs,
} from "@/hooks/acp-bridge-reducer"
import {
	activePermissionAtom,
	agentConnectedAtom,
	messagesAtom,
	promptInFlightAtom,
	sessionIdAtom,
	sessionStatusAtom,
} from "@/stores/atoms"
import type { PermissionRequest } from "@/types/acp"

beforeEach(() => {
	vi.stubGlobal("crypto", {
		randomUUID: () => "test-uuid",
	})
})

function createRefs(): AcpBridgeRefs {
	return {
		streaming: { current: "" },
		firstChunkLogged: { current: false },
	}
}

const permission: PermissionRequest = {
	requestId: "request-1",
	sessionId: "session-1",
	options: [{ optionId: "allow", name: "Allow" }],
}

describe("processAcpEvent", () => {
	it("opens the permission gate only for the active session", () => {
		const store = createStore()
		const refs = createRefs()
		store.set(sessionIdAtom, "session-1")

		processAcpEvent(store, refs, {
			type: "permission_request",
			payload: permission,
		})

		expect(store.get(activePermissionAtom)).toEqual(permission)
		expect(store.get(sessionStatusAtom)).toBe("awaiting_permission")

		processAcpEvent(store, refs, {
			type: "permission_request",
			payload: { ...permission, requestId: "request-2", sessionId: "other" },
		})

		expect(store.get(activePermissionAtom)).toEqual(permission)
	})

	it("clears the permission gate when the prompt completes", () => {
		const store = createStore()
		const refs = createRefs()
		store.set(sessionIdAtom, "session-1")
		store.set(activePermissionAtom, permission)
		store.set(promptInFlightAtom, true)
		store.set(sessionStatusAtom, "awaiting_permission")

		processAcpEvent(store, refs, {
			type: "prompt_complete",
			payload: { sessionId: "session-1" },
		})

		expect(store.get(activePermissionAtom)).toBeNull()
		expect(store.get(promptInFlightAtom)).toBe(false)
		expect(store.get(sessionStatusAtom)).toBe("idle")
	})

	it("ignores streaming updates from another session", () => {
		const store = createStore()
		const refs = createRefs()
		store.set(sessionIdAtom, "session-1")

		processAcpEvent(store, refs, {
			type: "session_update",
			payload: {
				sessionId: "other",
				update: {
					sessionUpdate: "agent_message_chunk",
					content: { type: "text", text: "ignored" },
				},
			},
		})

		expect(store.get(messagesAtom)).toEqual([])
		expect(store.get(promptInFlightAtom)).toBe(false)
	})

	it("streams active-session text immediately", () => {
		const store = createStore()
		const refs = createRefs()
		store.set(sessionIdAtom, "session-1")

		processAcpEvent(store, refs, {
			type: "session_update",
			payload: {
				sessionId: "session-1",
				update: {
					sessionUpdate: "agent_message_chunk",
					content: { type: "text", text: "Hello" },
				},
			},
		})

		expect(store.get(messagesAtom)[0]?.content).toBe("Hello")
		expect(store.get(promptInFlightAtom)).toBe(true)
		expect(store.get(sessionStatusAtom)).toBe("generating")
	})

	it("ignores session-scoped events without an active session", () => {
		const store = createStore()
		const refs = createRefs()

		processAcpEvent(store, refs, {
			type: "session_update",
			payload: {
				sessionId: "prewarmed-session",
				update: {
					sessionUpdate: "agent_message_chunk",
					content: { type: "text", text: "ignored" },
				},
			},
		})
		processAcpEvent(store, refs, {
			type: "permission_request",
			payload: permission,
		})

		expect(store.get(messagesAtom)).toEqual([])
		expect(store.get(activePermissionAtom)).toBeNull()
	})

	it("queues simultaneous permission requests for the same session", () => {
		const store = createStore()
		const refs = createRefs()
		store.set(sessionIdAtom, "session-1")

		const second: PermissionRequest = {
			...permission,
			requestId: "request-2",
		}
		processAcpEvent(store, refs, {
			type: "permission_request",
			payload: permission,
		})
		processAcpEvent(store, refs, {
			type: "permission_request",
			payload: second,
		})

		expect(store.get(activePermissionAtom)).toEqual(permission)
		expect(store.get(sessionStatusAtom)).toBe("awaiting_permission")
	})

	it("ignores session updates without a session id", () => {
		const store = createStore()
		const refs = createRefs()
		store.set(sessionIdAtom, "session-1")

		processAcpEvent(store, refs, {
			type: "session_update",
			payload: {
				update: {
					sessionUpdate: "agent_message_chunk",
					content: { type: "text", text: "ignored" },
				},
			},
		})

		expect(store.get(messagesAtom)).toEqual([])
	})

	it("ignores disconnects from an older connection", () => {
		const store = createStore()
		const refs = createRefs()
		processAcpEvent(store, refs, {
			type: "agent_ready",
			payload: {
				projectPath: "/current",
				capabilities: {
					loadSession: true,
					listSessions: false,
					resumeSession: false,
					closeSession: true,
					concurrentSessions: true,
				},
				connectionGeneration: 2,
			},
		})
		store.set(sessionIdAtom, "session-1")
		store.set(agentConnectedAtom, true)

		processAcpEvent(store, refs, {
			type: "disconnected",
			payload: { connectionGeneration: 1 },
		})

		expect(store.get(sessionIdAtom)).toBe("session-1")
		expect(store.get(agentConnectedAtom)).toBe(true)
	})

	it("clears the permission queue on disconnect", () => {
		const store = createStore()
		const refs = createRefs()
		store.set(sessionIdAtom, "session-1")
		processAcpEvent(store, refs, {
			type: "permission_request",
			payload: permission,
		})
		processAcpEvent(store, refs, {
			type: "permission_request",
			payload: { ...permission, requestId: "request-2" },
		})

		processAcpEvent(store, refs, {
			type: "disconnected",
			payload: {},
		})

		expect(store.get(activePermissionAtom)).toBeNull()
	})

	it("clears session and permission state on disconnect", () => {
		const store = createStore()
		const refs = createRefs()
		store.set(sessionIdAtom, "session-1")
		store.set(agentConnectedAtom, true)
		store.set(activePermissionAtom, permission)

		processAcpEvent(store, refs, {
			type: "disconnected",
			payload: {},
		})

		expect(store.get(sessionIdAtom)).toBeNull()
		expect(store.get(agentConnectedAtom)).toBe(false)
		expect(store.get(activePermissionAtom)).toBeNull()
		expect(store.get(sessionStatusAtom)).toBe("disconnected")
	})
})
