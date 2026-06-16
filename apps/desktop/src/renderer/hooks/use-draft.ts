import { atom, useAtomValue } from "jotai"
import { useCallback, useEffect, useMemo, useRef } from "react"
import { clearDraftAtom, draftsAtom, setDraftAtom } from "../atoms/preferences"
import { appStore } from "../atoms/store"

/** Key used for the new-chat (landing page) draft */
export const NEW_CHAT_DRAFT_KEY = "__new_chat__"

/**
 * Returns the current draft text for a given key (reactive).
 * Uses a per-key derived atom so changes to OTHER keys in the drafts map
 * do not cause re-renders. Only re-renders when this specific key's value changes.
 */
export function useDraft(key: string): string {
	const keyedAtom = useMemo(() => atom((get) => get(draftsAtom)[key] ?? ""), [key])
	return useAtomValue(keyedAtom)
}

/**
 * Returns the current draft text as a non-reactive snapshot.
 * Reads directly from the store without subscribing, so subsequent
 * draft writes do NOT cause re-renders. Recomputes only when `key` changes.
 *
 * Use this for `initialInput` props where the value is consumed once on mount
 * and reactive updates would cause unnecessary parent re-renders.
 */
export function useDraftSnapshot(key: string): string {
	return useMemo(() => appStore.get(draftsAtom)[key] ?? "", [key])
}

/**
 * Hook that returns a debounced setter for persisting draft text,
 * plus a clearDraft function for immediate cleanup.
 */
export function useDraftActions(key: string) {
	const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
	const latestTextRef = useRef<string | null>(null)
	const keyRef = useRef(key)
	keyRef.current = key

	const flush = useCallback(() => {
		if (timerRef.current !== null) {
			clearTimeout(timerRef.current)
			timerRef.current = null
		}
		if (latestTextRef.current !== null) {
			const text = latestTextRef.current
			latestTextRef.current = null
			if (text) {
				appStore.set(setDraftAtom, { key: keyRef.current, text })
			} else {
				appStore.set(clearDraftAtom, keyRef.current)
			}
		}
	}, [])

	const setDraft = useCallback(
		(text: string) => {
			latestTextRef.current = text
			if (timerRef.current !== null) {
				clearTimeout(timerRef.current)
			}
			timerRef.current = setTimeout(flush, 500)
		},
		[flush],
	)

	const clearDraft = useCallback(() => {
		if (timerRef.current !== null) {
			clearTimeout(timerRef.current)
			timerRef.current = null
		}
		latestTextRef.current = null
		appStore.set(clearDraftAtom, keyRef.current)
	}, [])

	// Flush pending draft on unmount
	useEffect(() => {
		return () => {
			flush()
		}
	}, [flush])

	return { setDraft, clearDraft }
}
