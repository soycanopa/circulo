import { useEffect, useRef } from "react"

function isModKey(event: KeyboardEvent): boolean {
	return event.metaKey || event.ctrlKey
}

function isTypingTarget(target: EventTarget | null): boolean {
	if (!(target instanceof HTMLElement)) return false
	const tag = target.tagName
	return (
		target.isContentEditable ||
		tag === "INPUT" ||
		tag === "TEXTAREA" ||
		tag === "SELECT"
	)
}

interface AppShortcutHandlers {
	onNewChat: () => void
	onOpenProject: () => void
	onOpenSettings: () => void
	onOpenCommandPalette: () => void
	onExportTranscript?: () => void
}

export function useAppShortcuts(handlers: AppShortcutHandlers) {
	const handlersRef = useRef(handlers)
	handlersRef.current = handlers

	useEffect(() => {
		function onKeyDown(event: KeyboardEvent) {
			const mod = isModKey(event)
			if (isTypingTarget(event.target) && !mod) return

			const key = event.key.toLowerCase()

			if (mod && key === "n") {
				event.preventDefault()
				handlersRef.current.onNewChat()
				return
			}

			if (mod && key === "k") {
				event.preventDefault()
				handlersRef.current.onOpenCommandPalette()
				return
			}

			if (mod && event.shiftKey && key === "e") {
				event.preventDefault()
				handlersRef.current.onExportTranscript?.()
			}
		}

		window.addEventListener("keydown", onKeyDown, { capture: true })
		return () => window.removeEventListener("keydown", onKeyDown, { capture: true })
	}, [])
}
