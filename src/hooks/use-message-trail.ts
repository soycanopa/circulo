import { useCallback, useEffect, useMemo, useRef } from "react"
import {
	createActiveTrailStore,
	deriveMessageTrailItems,
	deriveTrailAnchors,
	measureVisibleRowRange,
	resolveActiveTrailSnapshot,
} from "@/lib/message-trail.logic"
import type { ChatMessage } from "@/types/acp"

const TRAIL_FLASH_MS = 1200

export function useMessageTrail(messages: ChatMessage[]) {
	const scrollRef = useRef<HTMLDivElement>(null)
	const activeStoreRef = useRef(createActiveTrailStore())
	const activeStore = activeStoreRef.current

	const trailItems = useMemo(() => deriveMessageTrailItems(messages), [messages])
	const anchors = useMemo(() => deriveTrailAnchors(messages), [messages])
	const messageIds = useMemo(() => messages.map((message) => message.id), [messages])

	const scrollToMessage = useCallback((messageId: string) => {
		const scrollEl = scrollRef.current
		if (!scrollEl) {
			return
		}

		const el = scrollEl.querySelector(`[data-message-id="${messageId}"]`)
		if (!(el instanceof HTMLElement)) {
			return
		}

		el.scrollIntoView({ behavior: "smooth", block: "start" })
		el.dataset.trailFlash = "true"
		window.setTimeout(() => {
			delete el.dataset.trailFlash
		}, TRAIL_FLASH_MS)
	}, [])

	useEffect(() => {
		const scrollEl = scrollRef.current
		if (!scrollEl) {
			return
		}

		let rafId: number | null = null

		const update = () => {
			rafId = null
			const range = measureVisibleRowRange(scrollEl, messageIds)
			activeStore.set(
				resolveActiveTrailSnapshot(anchors, range.top, range.bottom),
			)
		}

		const schedule = () => {
			if (rafId === null) {
				rafId = requestAnimationFrame(update)
			}
		}

		schedule()
		scrollEl.addEventListener("scroll", schedule, { passive: true })

		const observer =
			typeof ResizeObserver !== "undefined"
				? new ResizeObserver(schedule)
				: null
		observer?.observe(scrollEl)

		return () => {
			scrollEl.removeEventListener("scroll", schedule)
			observer?.disconnect()
			if (rafId !== null) {
				cancelAnimationFrame(rafId)
			}
		}
	}, [activeStore, anchors, messageIds])

	return {
		scrollRef,
		activeStore,
		trailItems,
		scrollToMessage,
	}
}