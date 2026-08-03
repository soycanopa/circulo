import { getDefaultStore } from "jotai"
import { useEffect, useRef } from "react"
import {
	processAcpEvent,
	type AcpBridgeRefs,
} from "@/hooks/acp-bridge-reducer"
import { listenAcpEvents } from "@/lib/tauri"

export function useAcpBridge() {
	const streamingRef = useRef("")
	const firstChunkLogged = useRef(false)

	useEffect(() => {
		let cancelled = false
		let unlisteners: Array<() => void> = []
		const store = getDefaultStore()
		const refs: AcpBridgeRefs = {
			streaming: streamingRef,
			firstChunkLogged,
		}

		listenAcpEvents({
			onAgentReady: (payload) =>
				processAcpEvent(store, refs, { type: "agent_ready", payload }),
			onSessionReady: (payload) =>
				processAcpEvent(store, refs, { type: "session_ready", payload }),
			onProgress: (payload) =>
				processAcpEvent(store, refs, { type: "progress", payload }),
			onSessionUpdate: (payload) =>
				processAcpEvent(store, refs, { type: "session_update", payload }),
			onPermissionRequest: (payload) =>
				processAcpEvent(store, refs, {
					type: "permission_request",
					payload,
				}),
			onConfigOptions: (payload) =>
				processAcpEvent(store, refs, { type: "config_options", payload }),
			onPromptComplete: (payload) =>
				processAcpEvent(store, refs, { type: "prompt_complete", payload }),
			onError: (payload) =>
				processAcpEvent(store, refs, { type: "error", payload }),
			onDisconnected: (payload) =>
				processAcpEvent(store, refs, { type: "disconnected", payload }),
		}).then((list) => {
			if (cancelled) {
				for (const unlisten of list) unlisten()
				return
			}
			unlisteners = list
		})

		return () => {
			cancelled = true
			for (const unlisten of unlisteners) unlisten()
		}
	}, [])
}
