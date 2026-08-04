import { useRef, useState } from "react"

const HOVER_CLOSE_DELAY_MS = 120

export function useHoverPopover() {
	const [open, setOpen] = useState(false)
	const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

	function clearCloseTimer() {
		if (closeTimer.current) {
			clearTimeout(closeTimer.current)
			closeTimer.current = null
		}
	}

	function showPopover() {
		clearCloseTimer()
		setOpen(true)
	}

	function scheduleClose() {
		clearCloseTimer()
		closeTimer.current = setTimeout(() => {
			setOpen(false)
			closeTimer.current = null
		}, HOVER_CLOSE_DELAY_MS)
	}

	return { open, setOpen, showPopover, scheduleClose }
}
