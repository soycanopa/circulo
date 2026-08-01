import { useEffect } from "react"

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
	useEffect(() => {
		function onKeyDown(event: KeyboardEvent) {
			if (isTypingTarget(event.target) && !isModKey(event)) return

			if (isModKey(event) && event.key.toLowerCase() === "n") {
				event.preventDefault()
				handlers.onNewChat()
				return
			}

			if (isModKey(event) && event.key.toLowerCase() === "k") {
				event.preventDefault()
				handlers.onOpenCommandPalette()
				return
			}

			if (isModKey(event) && event.shiftKey && event.key.toLowerCase() === "e") {
				event.preventDefault()
				handlers.onExportTranscript?.()
			}
		}

		window.addEventListener("keydown", onKeyDown)
		return () => window.removeEventListener("keydown", onKeyDown)
	}, [handlers])
}
