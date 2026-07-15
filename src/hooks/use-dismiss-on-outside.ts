import { useEffect, type RefObject } from "react"

export function useDismissOnOutside(
	ref: RefObject<HTMLElement | null>,
	onDismiss: () => void,
	enabled: boolean,
) {
	useEffect(() => {
		if (!enabled) return

		function handlePointerDown(event: PointerEvent) {
			const target = event.target
			if (!(target instanceof Node)) return
			if (ref.current?.contains(target)) return
			onDismiss()
		}

		document.addEventListener("pointerdown", handlePointerDown, true)
		return () => document.removeEventListener("pointerdown", handlePointerDown, true)
	}, [enabled, onDismiss, ref])
}