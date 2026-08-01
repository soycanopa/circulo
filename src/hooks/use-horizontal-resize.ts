import { useCallback, type PointerEvent as ReactPointerEvent } from "react"

interface UseHorizontalResizeOptions {
	width: number
	onWidthChange: (width: number) => void
	min: number
	max: number
	/** When true, dragging left increases width (right-side panels). */
	invertDelta?: boolean
	onResizeStart?: () => void
	onResizeEnd?: () => void
}

export function useHorizontalResize({
	width,
	onWidthChange,
	min,
	max,
	invertDelta = false,
	onResizeStart,
	onResizeEnd,
}: UseHorizontalResizeOptions) {
	const onPointerDown = useCallback(
		(event: ReactPointerEvent<HTMLDivElement>) => {
			event.preventDefault()
			const startX = event.clientX
			const startWidth = width
			onResizeStart?.()

			const onMove = (moveEvent: PointerEvent) => {
				let delta = moveEvent.clientX - startX
				if (invertDelta) delta = -delta
				onWidthChange(Math.min(max, Math.max(min, startWidth + delta)))
			}

			const onUp = () => {
				document.removeEventListener("pointermove", onMove)
				document.removeEventListener("pointerup", onUp)
				onResizeEnd?.()
			}

			document.addEventListener("pointermove", onMove)
			document.addEventListener("pointerup", onUp)
		},
		[width, onWidthChange, min, max, invertDelta, onResizeStart, onResizeEnd],
	)

	return { onPointerDown }
}
