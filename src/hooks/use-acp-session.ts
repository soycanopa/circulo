import { useAtom, useSetAtom } from "jotai"
import { useCallback, useEffect, useRef } from "react"
import { applySessionUpdate } from "@/lib/acp-parser"
import { listenAcpEvents } from "@/lib/tauri"
import {
	activePermissionAtom,
	configOptionsAtom,
	errorMessageAtom,
	messagesAtom,
	projectPathAtom,
	sessionStatusAtom,
	streamingTextAtom,
} from "@/stores/atoms"
import type { PermissionRequest } from "@/types/acp"

export function useAcpSession() {
	const [messages, setMessages] = useAtom(messagesAtom)
	const [streamingText, setStreamingText] = useAtom(streamingTextAtom)
	const [sessionStatus, setSessionStatus] = useAtom(sessionStatusAtom)
	const [projectPath, setProjectPath] = useAtom(projectPathAtom)
	const setConfigOptions = useSetAtom(configOptionsAtom)
	const setActivePermission = useSetAtom(activePermissionAtom)
	const setErrorMessage = useSetAtom(errorMessageAtom)
	const streamingRef = useRef("")

	useEffect(() => {
		streamingRef.current = streamingText
	}, [streamingText])

	useEffect(() => {
		let unlisteners: Array<() => void> = []

		listenAcpEvents({
			onSessionReady: (payload) => {
				setProjectPath(payload.projectPath)
				setConfigOptions(payload.configOptions)
				setSessionStatus("idle")
				setErrorMessage(null)
			},
			onSessionUpdate: (payload) => {
				setSessionStatus("generating")
				setMessages((current) => {
					const result = applySessionUpdate(
						current,
						streamingRef.current,
						payload,
					)
					streamingRef.current = result.streamingText
					setStreamingText(result.streamingText)
					return result.messages
				})
			},
			onPermissionRequest: (payload) => {
				setSessionStatus("awaiting_permission")
				setActivePermission(payload as PermissionRequest)
			},
			onConfigOptions: (payload) => {
				setConfigOptions(payload.configOptions)
			},
			onPromptComplete: () => {
				streamingRef.current = ""
				setStreamingText("")
				setSessionStatus("idle")
			},
			onError: (payload) => {
				setErrorMessage(payload.message)
				setSessionStatus("idle")
			},
			onDisconnected: () => {
				setSessionStatus("disconnected")
				setProjectPath(null)
			},
		}).then((listeners) => {
			unlisteners = listeners
		})

		return () => {
			for (const unlisten of unlisteners) unlisten()
		}
	}, [
		setActivePermission,
		setConfigOptions,
		setErrorMessage,
		setMessages,
		setProjectPath,
		setSessionStatus,
		setStreamingText,
	])

	const resetConversation = useCallback(() => {
		streamingRef.current = ""
		setMessages([])
		setStreamingText("")
	}, [setMessages, setStreamingText])

	return {
		messages,
		streamingText,
		sessionStatus,
		projectPath,
		resetConversation,
	}
}