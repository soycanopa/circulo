import { createStore } from "jotai"
import { beforeEach, describe, expect, it, vi } from "vitest"
import {
	processAcpEvent,
	type AcpBridgeRefs,
} from "@/hooks/acp-bridge-reducer"
import {
	activePermissionAtom,
	activeSessionIdAtom,
	agentConnectedAtom,
	sessionsAtom,
} from "@/stores/atoms"
import type { PermissionRequest } from "@/types/acp"

beforeEach(() => {
	vi.stubGlobal("crypto", {
		randomUUID: () => "test-uuid",
	})
})

function createRefs(): AcpBridgeRefs {
	return {
		streaming: { current: new Map() },
		firstChunkLogged: { current: false },
	}
}

function activate(store: ReturnType<typeof createStore>, sessionId: string) {
	store.set(activeSessionIdAtom, sessionId)
	store.set(sessionsAtom, {
		...store.get(sessionsAtom),
		[sessionId]: {
			messages: [],
			streaming: "",
			promptInFlight: false,
			status: "idle",
			configOptions: [],
		},
	})
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
		activate(store, "session-1")

		processAcpEvent(store, refs, {
			type: "permission_request",
			payload: permission,
		})

		expect(store.get(activePermissionAtom)).toEqual(permission)
		expect(store.get(sessionsAtom)["session-1"]?.status).toBe("awaiting_permission")

		processAcpEvent(store, refs, {
			type: "permission_request",
			payload: { ...permission, requestId: "request-2", sessionId: "other" },
		})

		expect(store.get(activePermissionAtom)).toEqual(permission)
	})

	it("clears the permission gate when the prompt completes", () => {
		const store = createStore()
		const refs = createRefs()
		activate(store, "session-1")
		store.set(activePermissionAtom, permission)
		store.set(sessionsAtom, {
			...store.get(sessionsAtom),
			"session-1": {
				messages: [],
				streaming: "",
				promptInFlight: true,
				status: "awaiting_permission",
				configOptions: [],
			},
		})

		processAcpEvent(store, refs, {
			type: "prompt_complete",
			payload: { sessionId: "session-1" },
		})

		expect(store.get(activePermissionAtom)).toBeNull()
		expect(store.get(sessionsAtom)["session-1"]?.promptInFlight).toBe(false)
		expect(store.get(sessionsAtom)["session-1"]?.status).toBe("idle")
	})

	it("ignores streaming updates from another session", () => {
		const store = createStore()
		const refs = createRefs()
		activate(store, "session-1")

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

		expect(store.get(sessionsAtom)["other"]).toBeUndefined()
	})

	it("streams active-session text immediately", () => {
		const store = createStore()
		const refs = createRefs()
		activate(store, "session-1")

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

		expect(store.get(sessionsAtom)["session-1"]?.messages[0]?.content).toBe(
			"Hello",
		)
		expect(store.get(sessionsAtom)["session-1"]?.promptInFlight).toBe(true)
		expect(store.get(sessionsAtom)["session-1"]?.status).toBe("generating")
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

		expect(store.get(activePermissionAtom)).toBeNull()
	})

	it("queues simultaneous permission requests for the same session", () => {
		const store = createStore()
		const refs = createRefs()
		activate(store, "session-1")

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
		expect(store.get(sessionsAtom)["session-1"]?.status).toBe("awaiting_permission")
	})

	it("keeps concurrent sessions isolated", () => {
		const store = createStore()
		const refs = createRefs()
		activate(store, "session-a")
		// Pre-create the second session in the map so it counts as known, but keep
		// the visible pointer on session-a so updates must not bleed into the legacy atom.
		const sessions = store.get(sessionsAtom)
		store.set(sessionsAtom, {
			...sessions,
			"session-b": {
				messages: [],
				streaming: "",
				promptInFlight: false,
				status: "idle",
				configOptions: [],
			},
		})

		processAcpEvent(store, refs, {
			type: "session_update",
			payload: {
				sessionId: "session-b",
				update: {
					sessionUpdate: "agent_message_chunk",
					content: { type: "text", text: "from-b" },
				},
			},
		})

		expect(store.get(sessionsAtom)["session-b"]?.messages[0]?.content).toBe("from-b")
		expect(store.get(sessionsAtom)["session-b"]?.promptInFlight).toBe(true)
		expect(store.get(sessionsAtom)["session-a"]?.promptInFlight).toBe(false)
	})

	it("keeps streaming buffer per session", () => {
		const store = createStore()
		const refs = createRefs()
		activate(store, "session-a")
		activate(store, "session-b")

		// Send chunk to session-a while session-b is the active session.
		processAcpEvent(store, refs, {
			type: "session_update",
			payload: {
				sessionId: "session-a",
				update: {
					sessionUpdate: "agent_message_chunk",
					content: { type: "text", text: "for-a" },
				},
			},
		})
		processAcpEvent(store, refs, {
			type: "session_update",
			payload: {
				sessionId: "session-b",
				update: {
					sessionUpdate: "agent_message_chunk",
					content: { type: "text", text: "for-b" },
				},
			},
		})

		// prompt_complete on session-a must not disturb session-b's accumulated buffer.
		processAcpEvent(store, refs, {
			type: "prompt_complete",
			payload: { sessionId: "session-a" },
		})

		expect(refs.streaming.current.has("session-a")).toBe(false)
		expect(refs.streaming.current.has("session-b")).toBe(false)
		expect(store.get(sessionsAtom)["session-a"]?.messages).toHaveLength(1)
		expect(store.get(sessionsAtom)["session-a"]?.promptInFlight).toBe(false)
		expect(store.get(sessionsAtom)["session-b"]?.promptInFlight).toBe(true)
	})

	it("ignores session updates without a session id", () => {
		const store = createStore()
		const refs = createRefs()
		activate(store, "session-1")

		processAcpEvent(store, refs, {
			type: "session_update",
			payload: {
				update: {
					sessionUpdate: "agent_message_chunk",
					content: { type: "text", text: "ignored" },
				},
			},
		})

		expect(store.get(sessionsAtom)["session-1"]?.messages).toEqual([])
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
		activate(store, "session-1")
		store.set(agentConnectedAtom, true)

		processAcpEvent(store, refs, {
			type: "disconnected",
			payload: { connectionGeneration: 1 },
		})

		expect(store.get(activeSessionIdAtom)).toBe("session-1")
		expect(store.get(agentConnectedAtom)).toBe(true)
	})

	it("clears the permission queue on disconnect", () => {
		const store = createStore()
		const refs = createRefs()
		activate(store, "session-1")
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
		activate(store, "session-1")
		store.set(agentConnectedAtom, true)
		store.set(activePermissionAtom, permission)

		processAcpEvent(store, refs, {
			type: "disconnected",
			payload: {},
		})

		expect(store.get(activeSessionIdAtom)).toBeNull()
		expect(store.get(agentConnectedAtom)).toBe(false)
		expect(store.get(activePermissionAtom)).toBeNull()
	})
})
