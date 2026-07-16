import type { ChatMessage } from "@/types/acp"

export interface MessageTrailItem {
	id: string
	ordinal: number
	preview: string
	responsePreview: string
}

export interface MessageTrailAnchor {
	id: string
	rowIndex: number
}

export interface ActiveTrailSnapshot {
	currentId: string | null
	visibleIds: readonly string[]
}

export const EMPTY_ACTIVE_TRAIL_SNAPSHOT: ActiveTrailSnapshot = {
	currentId: null,
	visibleIds: [],
}

const MAX_PREVIEW_LENGTH = 280

function normalizePreview(text: string): string {
	const collapsed = text.replace(/\s+/g, " ").trim()
	return collapsed.length > MAX_PREVIEW_LENGTH
		? `${collapsed.slice(0, MAX_PREVIEW_LENGTH).trimEnd()}…`
		: collapsed
}

export function deriveMessageTrailItems(
	messages: readonly ChatMessage[],
): MessageTrailItem[] {
	const items: MessageTrailItem[] = []
	let currentTurnIndex = -1

	for (const message of messages) {
		if (message.role === "user") {
			items.push({
				id: message.id,
				ordinal: items.length + 1,
				preview: normalizePreview(message.content),
				responsePreview: "",
			})
			currentTurnIndex = items.length - 1
			continue
		}

		if (
			message.role === "assistant" &&
			currentTurnIndex >= 0 &&
			message.kind !== "auth-request"
		) {
			const responsePreview = normalizePreview(message.content)
			if (responsePreview !== "") {
				items[currentTurnIndex]!.responsePreview = responsePreview
			}
		}
	}

	return items
}

export function deriveTrailAnchors(
	messages: readonly Pick<ChatMessage, "id" | "role">[],
): MessageTrailAnchor[] {
	const anchors: MessageTrailAnchor[] = []
	messages.forEach((message, rowIndex) => {
		if (message.role === "user") {
			anchors.push({ id: message.id, rowIndex })
		}
	})
	return anchors
}

export function resolveActiveTrailMessageId(
	anchors: readonly MessageTrailAnchor[],
	topVisibleRowIndex: number,
): string | null {
	if (anchors.length === 0) {
		return null
	}

	let activeId: string = anchors[0]!.id
	for (const anchor of anchors) {
		if (anchor.rowIndex <= topVisibleRowIndex) {
			activeId = anchor.id
		} else {
			break
		}
	}
	return activeId
}

function areIdListsEqual(a: readonly string[], b: readonly string[]): boolean {
	if (a.length !== b.length) {
		return false
	}
	for (let i = 0; i < a.length; i += 1) {
		if (a[i] !== b[i]) {
			return false
		}
	}
	return true
}

export function areActiveTrailSnapshotsEqual(
	a: ActiveTrailSnapshot,
	b: ActiveTrailSnapshot,
): boolean {
	return a.currentId === b.currentId && areIdListsEqual(a.visibleIds, b.visibleIds)
}

export function resolveActiveTrailSnapshot(
	anchors: readonly MessageTrailAnchor[],
	topVisibleRowIndex: number,
	bottomVisibleRowIndex: number,
): ActiveTrailSnapshot {
	if (anchors.length === 0 || !Number.isFinite(topVisibleRowIndex)) {
		return EMPTY_ACTIVE_TRAIL_SNAPSHOT
	}

	const currentId = resolveActiveTrailMessageId(anchors, topVisibleRowIndex)
	const bottomRowIndex = Number.isFinite(bottomVisibleRowIndex)
		? Math.max(topVisibleRowIndex, bottomVisibleRowIndex)
		: topVisibleRowIndex

	const visibleIds: string[] = []
	for (const anchor of anchors) {
		if (anchor.rowIndex < topVisibleRowIndex) {
			continue
		}
		if (anchor.rowIndex > bottomRowIndex) {
			break
		}
		visibleIds.push(anchor.id)
	}

	return visibleIds.length === 0 && currentId === null
		? EMPTY_ACTIVE_TRAIL_SNAPSHOT
		: { currentId, visibleIds }
}

export interface ActiveTrailStore {
	get: () => ActiveTrailSnapshot
	set: (value: ActiveTrailSnapshot | null) => void
	subscribe: (listener: () => void) => () => void
}

export function createActiveTrailStore(): ActiveTrailStore {
	let current: ActiveTrailSnapshot = EMPTY_ACTIVE_TRAIL_SNAPSHOT
	const listeners = new Set<() => void>()

	return {
		get: () => current,
		set: (value) => {
			const next = value ?? EMPTY_ACTIVE_TRAIL_SNAPSHOT
			if (areActiveTrailSnapshotsEqual(next, current)) {
				return
			}
			current = next
			for (const listener of listeners) {
				listener()
			}
		},
		subscribe: (listener) => {
			listeners.add(listener)
			return () => {
				listeners.delete(listener)
			}
		},
	}
}

export function clampNumber(value: number, min: number, max: number): number {
	if (!Number.isFinite(value)) {
		return min
	}
	if (max < min) {
		return min
	}
	return value < min ? min : value > max ? max : value
}

export interface TrailGeometry {
	startY: number
	spacing: number
	centerYs: number[]
	contentHeight: number
}

export function computeTrailGeometry(input: {
	count: number
	spacingPx?: number
	paddingPx?: number
}): TrailGeometry | null {
	const count = input.count
	const spacing = count <= 1 ? 0 : (input.spacingPx ?? 10)
	const padding = input.paddingPx ?? 12

	if (count <= 0) {
		return null
	}

	const centerYs: number[] = []
	for (let i = 0; i < count; i += 1) {
		centerYs.push(padding + i * spacing)
	}

	return {
		startY: padding,
		spacing,
		centerYs,
		contentHeight: 2 * padding + (count - 1) * spacing,
	}
}

export function computeSigma(spacing: number): number {
	return clampNumber(spacing * 1.5, Math.min(spacing * 2, 8), 22)
}

export function computeGaussianWeights(
	centerYs: readonly number[],
	pointerY: number,
	sigma: number,
): number[] {
	if (sigma <= 0) {
		return centerYs.map((centerY) => (centerY === pointerY ? 1 : 0))
	}

	const twoSigmaSquared = 2 * sigma * sigma
	return centerYs.map((centerY) => {
		const distance = centerY - pointerY
		return Math.exp(-(distance * distance) / twoSigmaSquared)
	})
}

export interface TickStyle {
	width: number
	opacity: number
}

export function computeTickStyles(
	weights: readonly number[],
	currentAnchorIndex: number | null,
	baseW: number,
	effectiveMaxW: number,
	restOpacity: number,
	anchorOpacity: number,
): TickStyle[] {
	return weights.map((weight, index) => ({
		width: baseW + (effectiveMaxW - baseW) * weight,
		opacity: index === currentAnchorIndex ? anchorOpacity : restOpacity,
	}))
}

export function computeRestStyles(
	count: number,
	currentAnchorIndex: number | null,
	baseW: number,
	restOpacity: number,
	anchorOpacity: number,
): TickStyle[] {
	const styles: TickStyle[] = []
	for (let i = 0; i < count; i += 1) {
		styles.push({
			width: baseW,
			opacity: i === currentAnchorIndex ? anchorOpacity : restOpacity,
		})
	}
	return styles
}

export function computeFocusedIndex(pointerY: number, geometry: TrailGeometry): number {
	const count = geometry.centerYs.length
	if (count <= 1 || geometry.spacing === 0) {
		return 0
	}
	if (!Number.isFinite(pointerY)) {
		return 0
	}

	const endY = geometry.startY + (count - 1) * geometry.spacing
	const clampedY = clampNumber(pointerY, geometry.startY, endY)
	const raw = Math.round((clampedY - geometry.startY) / geometry.spacing)
	return clampNumber(raw, 0, count - 1)
}

export function clampTooltipTop(
	centerY: number,
	tooltipH: number,
	railH: number,
	margin = 4,
): number {
	const half = tooltipH / 2 + margin
	return clampNumber(centerY, half, Math.max(half, railH - half))
}

export function measureVisibleRowRange(
	scrollEl: HTMLElement,
	messageIds: readonly string[],
): { top: number; bottom: number } {
	const rect = scrollEl.getBoundingClientRect()
	let top = Number.POSITIVE_INFINITY
	let bottom = -1

	for (let index = 0; index < messageIds.length; index += 1) {
		const id = messageIds[index]!
		const el = scrollEl.querySelector(`[data-message-id="${id}"]`)
		if (!el) {
			continue
		}
		const elRect = el.getBoundingClientRect()
		if (elRect.bottom > rect.top && elRect.top < rect.bottom) {
			top = Math.min(top, index)
			bottom = Math.max(bottom, index)
		}
	}

	if (top === Number.POSITIVE_INFINITY) {
		return { top: 0, bottom: Math.max(0, messageIds.length - 1) }
	}

	return { top, bottom }
}