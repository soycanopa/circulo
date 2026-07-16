import { useEffect, useRef, type RefObject } from "react"
import type { ChatMessage } from "@/types/acp"

const BOTTOM_THRESHOLD_PX = 96

interface UseChatAutoscrollOptions {
	streamingText: string
	messages: ChatMessage[]
	promptInFlight: boolean
}

/** Keeps the chat pinned to the bottom while streaming unless the user scrolls up. */
export function useChatAutoscroll(
	scrollRef: RefObject<HTMLDivElement | null>,
	{ streamingText, messages, promptInFlight }: UseChatAutoscrollOptions,
) {
	const pinnedRef = useRef(true)

	useEffect(() => {
		const el = scrollRef.current
		if (!el) return

		const onScroll = () => {
			const distanceFromBottom =
				el.scrollHeight - el.scrollTop - el.clientHeight
			pinnedRef.current = distanceFromBottom <= BOTTOM_THRESHOLD_PX
		}

		onScroll()
		el.addEventListener("scroll", onScroll, { passive: true })
		return () => el.removeEventListener("scroll", onScroll)
	}, [scrollRef])

	useEffect(() => {
		if (!pinnedRef.current && !promptInFlight) return
		const el = scrollRef.current
		if (!el) return

		const frame = requestAnimationFrame(() => {
			el.scrollTop = el.scrollHeight
		})
		return () => cancelAnimationFrame(frame)
	}, [scrollRef, streamingText, messages, promptInFlight])
}