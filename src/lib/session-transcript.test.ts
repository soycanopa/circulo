import { createStore } from "jotai"
import { beforeEach, describe, expect, it, vi } from "vitest"
import {
	getLiveSessionMessages,
	hydrateSessionFromDisk,
	isLiveSessionPristine,
} from "@/lib/session-transcript"
import { sessionsAtom } from "@/stores/atoms"
import type { ChatMessage, StoredTranscript } from "@/types/acp"

vi.mock("@/lib/tauri", () => ({
	loadChatTranscript: vi.fn(),
}))

import { loadChatTranscript } from "@/lib/tauri"

const mockLoad = loadChatTranscript as ReturnType<typeof vi.fn>

const sampleMessages: ChatMessage[] = [
	{ id: "1", role: "user", content: "hello", toolCalls: [], timestamp: 1 },
]

function storedTranscript(
	messages: StoredTranscript["messages"],
): StoredTranscript {
	return {
		sessionId: "s-1",
		projectPath: "/proj",
		title: "Chat",
		createdAt: 1,
		updatedAt: 1,
		messages,
	}
}

describe("session-transcript", () => {
	beforeEach(() => {
		mockLoad.mockReset()
	})

	it("isLiveSessionPristine is false when live messages exist", async () => {
		const store = createStore()
		store.set(sessionsAtom, {
			"s-1": {
				messages: sampleMessages,
				streaming: "",
				promptInFlight: false,
				status: "idle",
				configOptions: [],
				contextUsage: null,
			},
		})

		await expect(isLiveSessionPristine(store, "/proj", "s-1")).resolves.toBe(
			false,
		)
		expect(mockLoad).not.toHaveBeenCalled()
	})

	it("isLiveSessionPristine checks disk when live buffer is empty", async () => {
		const store = createStore()
		mockLoad.mockResolvedValue(storedTranscript(sampleMessages))

		await expect(isLiveSessionPristine(store, "/proj", "s-1")).resolves.toBe(
			false,
		)
	})

	it("isLiveSessionPristine is true when live and disk are empty", async () => {
		const store = createStore()
		mockLoad.mockResolvedValue(storedTranscript([]))

		await expect(isLiveSessionPristine(store, "/proj", "s-1")).resolves.toBe(
			true,
		)
	})

	it("hydrateSessionFromDisk fills an empty session slot", async () => {
		const store = createStore()
		mockLoad.mockResolvedValue(storedTranscript(sampleMessages))

		const hydrated = await hydrateSessionFromDisk(store, "/proj", "s-1")
		expect(hydrated).toEqual(sampleMessages)
		expect(getLiveSessionMessages(store, "s-1")).toEqual(sampleMessages)
	})

	it("hydrateSessionFromDisk skips disk when live messages already exist", async () => {
		const store = createStore()
		store.set(sessionsAtom, {
			"s-1": {
				messages: sampleMessages,
				streaming: "",
				promptInFlight: false,
				status: "idle",
				configOptions: [],
				contextUsage: null,
			},
		})

		const hydrated = await hydrateSessionFromDisk(store, "/proj", "s-1")
		expect(hydrated).toEqual(sampleMessages)
		expect(mockLoad).not.toHaveBeenCalled()
	})
})
