import { atom } from "jotai"
import { atomFamily } from "jotai-family"
import type { Part } from "../lib/types"

// ============================================================
// First-seen timestamps for tool parts
// ============================================================

/**
 * Module-scoped map tracking when each tool part was first observed by the
 * client. This records the wall-clock time from when the tool card first
 * appeared (typically at the "pending" state, when the LLM starts streaming
 * tool arguments) rather than the server-side execution start time.
 *
 * Used by `getToolDuration()` to show wall-clock durations that match
 * what the user actually experiences, falling back to the server-side
 * `state.time.start` for parts loaded from REST (page refresh, reconnect).
 */
const partFirstSeenAt = new Map<string, number>()

/** Record the first-seen timestamp for a part, only if not already tracked. */
function trackFirstSeen(part: Part): void {
	if (part.type === "tool" && !partFirstSeenAt.has(part.id)) {
		partFirstSeenAt.set(part.id, Date.now())
	}
}

/** Look up the first-seen timestamp for a part. */
export function getPartFirstSeenAt(partId: string): number | undefined {
	return partFirstSeenAt.get(partId)
}

// ============================================================
// Helpers
// ============================================================

function binarySearch<T>(
	arr: T[],
	target: string,
	key: (item: T) => string,
): { found: boolean; index: number } {
	let lo = 0
	let hi = arr.length
	while (lo < hi) {
		const mid = (lo + hi) >>> 1
		const cmp = key(arr[mid]).localeCompare(target)
		if (cmp < 0) lo = mid + 1
		else if (cmp > 0) hi = mid
		else return { found: true, index: mid }
	}
	return { found: false, index: lo }
}

// ============================================================
// Per-message part list (sorted by id)
// ============================================================

export const partsFamily = atomFamily((_messageId: string) => atom<Part[]>([]))

// ============================================================
// Action atoms
// ============================================================

/** Upsert a single part */
export const upsertPartAtom = atom(null, (get, set, part: Part) => {
	trackFirstSeen(part)
	const messageId = part.messageID
	const existing = get(partsFamily(messageId))

	// Fast path: no parts yet for this message
	if (!existing || existing.length === 0) {
		set(partsFamily(messageId), [part])
		return
	}

	const result = binarySearch(existing, part.id, (p) => p.id)

	if (result.found) {
		if (existing[result.index] === part) return // reference equality skip
		const updated = existing.slice()
		updated[result.index] = part
		set(partsFamily(messageId), updated)
	} else {
		const updated = existing.slice()
		updated.splice(result.index, 0, part)
		set(partsFamily(messageId), updated)
	}
})

/** Batch upsert multiple parts (used when flushing streaming buffer) */
export const batchUpsertPartsAtom = atom(null, (get, set, parts: Part[]) => {
	if (parts.length === 0) return

	// Group by messageId to minimize atom writes
	const byMessage = new Map<string, Part[]>()
	for (const part of parts) {
		const group = byMessage.get(part.messageID) ?? []
		group.push(part)
		byMessage.set(part.messageID, group)
	}

	for (const [messageId, messageParts] of byMessage) {
		const existing = get(partsFamily(messageId))
		const updated = existing ? [...existing] : []

		for (const part of messageParts) {
			trackFirstSeen(part)
			const result = binarySearch(updated, part.id, (p) => p.id)
			if (result.found) {
				updated[result.index] = part
			} else {
				updated.splice(result.index, 0, part)
			}
		}

		set(partsFamily(messageId), updated)
	}
})

/** Apply a string delta to a specific field of an existing part */
export const applyPartDeltaAtom = atom(
	null,
	(
		get,
		set,
		args: {
			messageId: string
			partId: string
			field: string
			delta: string
		},
	) => {
		const existing = get(partsFamily(args.messageId))
		if (!existing || existing.length === 0) return
		const result = binarySearch(existing, args.partId, (p) => p.id)
		if (!result.found) return
		const part = existing[result.index]
		// Part is a discriminated union; the server sends the field name as a plain string.
		// We read the current value and append the delta, defaulting to empty string.
		const record = part as Record<string, unknown>
		const current = record[args.field]
		const updated = existing.slice()
		updated[result.index] = {
			...part,
			[args.field]: (typeof current === "string" ? current : "") + args.delta,
		}
		set(partsFamily(args.messageId), updated)
	},
)

/** Remove a part from a message */
export const removePartAtom = atom(
	null,
	(
		get,
		set,
		args: {
			messageId: string
			partId: string
		},
	) => {
		const existing = get(partsFamily(args.messageId))
		if (!existing) return
		const result = binarySearch(existing, args.partId, (p) => p.id)
		if (!result.found) return
		const updated = [...existing]
		updated.splice(result.index, 1)
		set(partsFamily(args.messageId), updated)
	},
)
