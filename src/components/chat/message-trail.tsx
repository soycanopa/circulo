import {
	useCallback,
	useEffect,
	useId,
	useMemo,
	useRef,
	useState,
	useSyncExternalStore,
	type FocusEvent as ReactFocusEvent,
	type KeyboardEvent as ReactKeyboardEvent,
	type MouseEvent as ReactMouseEvent,
	type PointerEvent as ReactPointerEvent,
} from "react"
import { cn } from "@/lib/utils"
import {
	clampNumber,
	clampTooltipTop,
	computeFocusedIndex,
	computeGaussianWeights,
	computeRestStyles,
	computeSigma,
	computeTickStyles,
	computeTrailGeometry,
	type ActiveTrailStore,
	type MessageTrailItem,
	type TickStyle,
	type TrailGeometry,
} from "@/lib/message-trail.logic"

interface MessageTrailProps {
	items: readonly MessageTrailItem[]
	activeStore: ActiveTrailStore
	onSelect: (messageId: string) => void
}

const MIN_PANE_WIDTH_PX = 864
const RAIL_WIDTH_PX = 56
const RAIL_MAX_HEIGHT_RATIO = 0.8
const TICK_LEFT_PAD_PX = 14
const TICK_HEIGHT_PX = 2
const TICK_BASE_W = 6
const TICK_MAX_W = 30
const TICK_SPACING_PX = 10
const TICK_REST_OPACITY = 0.2
const TICK_VISIBLE_OPACITY = 0.52
const TICK_ANCHOR_OPACITY = 0.9
const TICK_FOCUS_OPACITY = 1
const TOOLTIP_ESTIMATED_H_PX = 56
const TOOLTIP_OFFSET_X_PX = 8

export function MessageTrail({ items, activeStore, onSelect }: MessageTrailProps) {
	const rootRef = useRef<HTMLElement | null>(null)
	const viewportRef = useRef<HTMLDivElement | null>(null)
	const tooltipRef = useRef<HTMLDivElement | null>(null)
	const tooltipMessageRef = useRef<HTMLDivElement | null>(null)
	const tooltipResponseRef = useRef<HTMLDivElement | null>(null)
	const tickRefs = useRef<(HTMLButtonElement | null)[]>([])
	const tooltipId = useId()

	const [hasGutter, setHasGutter] = useState(false)
	const [rovingIndex, setRovingIndex] = useState(0)

	const trailSnapshot = useSyncExternalStore(
		activeStore.subscribe,
		activeStore.get,
		activeStore.get,
	)

	const anchorIndex = useMemo(
		() => items.findIndex((item) => item.id === trailSnapshot.currentId),
		[items, trailSnapshot.currentId],
	)

	const visibleIndexes = useMemo(() => {
		if (trailSnapshot.visibleIds.length === 0) {
			return []
		}
		const visibleIds = new Set(trailSnapshot.visibleIds)
		const indexes: number[] = []
		items.forEach((item, index) => {
			if (visibleIds.has(item.id)) {
				indexes.push(index)
			}
		})
		return indexes
	}, [items, trailSnapshot.visibleIds])

	const visibleIndexSet = useMemo(() => new Set(visibleIndexes), [visibleIndexes])
	const visible = hasGutter && items.length > 1

	const geometry = useMemo(
		() => computeTrailGeometry({ count: items.length, spacingPx: TICK_SPACING_PX }),
		[items.length],
	)

	const rafIdRef = useRef<number | null>(null)
	const latestPointerClientYRef = useRef<number | null>(null)
	const focusOverrideIndexRef = useRef<number | null>(null)
	const geometryRef = useRef<TrailGeometry | null>(geometry)
	geometryRef.current = geometry
	const viewportTopRef = useRef(0)
	const tooltipIndexRef = useRef(-1)
	const reducedMotionRef = useRef(false)
	const itemsRef = useRef(items)
	itemsRef.current = items
	const anchorIndexRef = useRef(anchorIndex)
	anchorIndexRef.current = anchorIndex
	const visibleIndexesRef = useRef(visibleIndexes)
	visibleIndexesRef.current = visibleIndexes
	const onSelectRef = useRef(onSelect)
	onSelectRef.current = onSelect
	const visibleRef = useRef(visible)
	visibleRef.current = visible

	if (tickRefs.current.length !== items.length) {
		tickRefs.current = Array.from<HTMLButtonElement | null>({
			length: items.length,
		}).fill(null)
	}

	const writeStyles = useCallback((styles: readonly TickStyle[]) => {
		const refs = tickRefs.current
		for (let i = 0; i < styles.length; i += 1) {
			const el = refs[i]
			if (!el) {
				continue
			}
			el.style.width = `${styles[i]!.width}px`
			el.style.opacity = `${styles[i]!.opacity}`
		}
	}, [])

	const hideTooltip = useCallback(() => {
		tooltipIndexRef.current = -1
		const tip = tooltipRef.current
		if (tip) {
			tip.style.visibility = "hidden"
		}
	}, [])

	const showTooltip = useCallback((index: number, geometryValue: TrailGeometry) => {
		const tip = tooltipRef.current
		const item = itemsRef.current[index]
		if (!tip || !item) {
			return
		}

		if (tooltipIndexRef.current !== index) {
			tooltipIndexRef.current = index
			const messageEl = tooltipMessageRef.current
			const responseEl = tooltipResponseRef.current
			if (messageEl) {
				messageEl.textContent = item.preview
			}
			if (responseEl) {
				responseEl.textContent = item.responsePreview
				responseEl.style.display = item.responsePreview ? "" : "none"
			}
		}

		const viewport = viewportRef.current
		const viewportHeight = viewport?.clientHeight ?? 0
		const tooltipHeight = tip.offsetHeight || TOOLTIP_ESTIMATED_H_PX
		const centerY = geometryValue.centerYs[index] ?? viewportHeight / 2
		const visibleY = centerY - (viewport?.scrollTop ?? 0)
		const offsetTop = viewport?.offsetTop ?? 0
		tip.style.top = `${offsetTop + clampTooltipTop(visibleY, tooltipHeight, viewportHeight)}px`
		tip.style.visibility = "visible"
	}, [])

	const applyHighlightFloors = useCallback((styles: TickStyle[]) => {
		const anchorIndexValue = anchorIndexRef.current
		for (const index of visibleIndexesRef.current) {
			const style = styles[index]
			if (style) {
				style.opacity = Math.max(style.opacity, TICK_VISIBLE_OPACITY)
			}
		}
		const anchorStyle =
			anchorIndexValue >= 0 ? styles[anchorIndexValue] : undefined
		if (anchorStyle) {
			anchorStyle.opacity = Math.max(anchorStyle.opacity, TICK_ANCHOR_OPACITY)
		}
	}, [])

	const applyRest = useCallback(() => {
		const styles = computeRestStyles(
			itemsRef.current.length,
			anchorIndexRef.current,
			TICK_BASE_W,
			TICK_REST_OPACITY,
			TICK_ANCHOR_OPACITY,
		)
		applyHighlightFloors(styles)
		writeStyles(styles)
		hideTooltip()
	}, [applyHighlightFloors, hideTooltip, writeStyles])

	const layoutTicks = useCallback(() => {
		const geometryValue = geometryRef.current
		if (!geometryValue) {
			return
		}

		const refs = tickRefs.current
		for (let i = 0; i < refs.length; i += 1) {
			const el = refs[i]
			if (!el) {
				continue
			}
			const centerY = geometryValue.centerYs[i] ?? 0
			el.style.top = `${centerY - TICK_HEIGHT_PX / 2}px`
		}

		if (
			latestPointerClientYRef.current === null &&
			focusOverrideIndexRef.current === null
		) {
			applyRest()
		}
	}, [applyRest])

	const renderFrame = useCallback(() => {
		rafIdRef.current = null
		const geometryValue = geometryRef.current
		if (!geometryValue || !visibleRef.current) {
			return
		}

		const count = itemsRef.current.length
		if (count === 0) {
			return
		}

		let activeY: number | null = null
		const rawPointerY = latestPointerClientYRef.current
		if (rawPointerY !== null) {
			activeY = rawPointerY + (viewportRef.current?.scrollTop ?? 0)
		} else if (focusOverrideIndexRef.current !== null) {
			activeY = geometryValue.centerYs[focusOverrideIndexRef.current] ?? null
		}

		if (activeY === null) {
			applyRest()
			return
		}

		const anchor = anchorIndexRef.current
		const focusedIndex = computeFocusedIndex(activeY, geometryValue)

		let styles: TickStyle[]
		if (geometryValue.spacing === 0 || reducedMotionRef.current) {
			styles = computeRestStyles(
				count,
				anchor,
				TICK_BASE_W,
				TICK_REST_OPACITY,
				TICK_ANCHOR_OPACITY,
			)
			const focusedStyle = styles[focusedIndex]
			if (focusedStyle) {
				focusedStyle.width = TICK_MAX_W
			}
		} else {
			const sigma = computeSigma(geometryValue.spacing)
			const weights = computeGaussianWeights(geometryValue.centerYs, activeY, sigma)
			styles = computeTickStyles(
				weights,
				anchor,
				TICK_BASE_W,
				TICK_MAX_W,
				TICK_REST_OPACITY,
				TICK_ANCHOR_OPACITY,
			)
		}

		applyHighlightFloors(styles)
		const focusedStyle = styles[focusedIndex]
		if (focusedStyle) {
			focusedStyle.opacity = TICK_FOCUS_OPACITY
		}
		writeStyles(styles)
		showTooltip(focusedIndex, geometryValue)
	}, [applyHighlightFloors, applyRest, showTooltip, writeStyles])

	const scheduleFrame = useCallback(() => {
		if (rafIdRef.current === null) {
			rafIdRef.current = requestAnimationFrame(renderFrame)
		}
	}, [renderFrame])

	const cancelFrame = useCallback(() => {
		if (rafIdRef.current !== null) {
			cancelAnimationFrame(rafIdRef.current)
			rafIdRef.current = null
		}
	}, [])

	useEffect(() => {
		const root = rootRef.current
		const pane = root?.parentElement
		if (!pane || typeof ResizeObserver === "undefined") {
			return
		}

		let pendingRaf: number | null = null
		const measure = () => {
			pendingRaf = null
			setHasGutter(pane.clientWidth >= MIN_PANE_WIDTH_PX)
		}
		const schedule = () => {
			if (pendingRaf === null) {
				pendingRaf = requestAnimationFrame(measure)
			}
		}

		schedule()
		const observer = new ResizeObserver(schedule)
		observer.observe(pane)

		return () => {
			if (pendingRaf !== null) {
				cancelAnimationFrame(pendingRaf)
			}
			observer.disconnect()
		}
	}, [])

	useEffect(() => {
		layoutTicks()
	}, [geometry, layoutTicks])

	useEffect(() => {
		if (
			latestPointerClientYRef.current === null &&
			focusOverrideIndexRef.current === null
		) {
			applyRest()
		}
	}, [anchorIndex, applyRest, visibleIndexes])

	useEffect(() => {
		reducedMotionRef.current =
			typeof window !== "undefined" && typeof window.matchMedia === "function"
				? window.matchMedia("(prefers-reduced-motion: reduce)").matches
				: false
	}, [])

	useEffect(() => {
		if (!visible) {
			cancelFrame()
			latestPointerClientYRef.current = null
			focusOverrideIndexRef.current = null
			hideTooltip()
		}
	}, [visible, cancelFrame, hideTooltip])

	useEffect(() => cancelFrame, [cancelFrame])

	const handlePointerMove = useCallback(
		(event: ReactPointerEvent<HTMLDivElement>) => {
			if (event.pointerType === "touch" || !visibleRef.current) {
				return
			}
			latestPointerClientYRef.current = event.clientY - viewportTopRef.current
			scheduleFrame()
		},
		[scheduleFrame],
	)

	const handlePointerEnter = useCallback(
		(event: ReactPointerEvent<HTMLDivElement>) => {
			if (event.pointerType === "touch" || !visibleRef.current) {
				return
			}
			const rect = viewportRef.current?.getBoundingClientRect()
			if (rect) {
				viewportTopRef.current = rect.top
			}
			latestPointerClientYRef.current = event.clientY - viewportTopRef.current
			scheduleFrame()
		},
		[scheduleFrame],
	)

	const handlePointerLeave = useCallback(
		(event: ReactPointerEvent<HTMLDivElement>) => {
			if (event.pointerType === "touch") {
				return
			}
			latestPointerClientYRef.current = null
			cancelFrame()
			if (focusOverrideIndexRef.current !== null) {
				scheduleFrame()
			} else {
				applyRest()
			}
		},
		[applyRest, cancelFrame, scheduleFrame],
	)

	const handleScroll = useCallback(() => {
		if (
			latestPointerClientYRef.current !== null ||
			focusOverrideIndexRef.current !== null
		) {
			scheduleFrame()
		}
	}, [scheduleFrame])

	const handleClick = useCallback((event: ReactMouseEvent<HTMLDivElement>) => {
		const geometryValue = geometryRef.current
		const viewport = viewportRef.current
		if (!geometryValue || !viewport) {
			return
		}

		const contentY =
			event.clientY - viewport.getBoundingClientRect().top + viewport.scrollTop
		const index = computeFocusedIndex(contentY, geometryValue)
		const item = itemsRef.current[index]
		if (item) {
			onSelectRef.current(item.id)
		}
	}, [])

	const focusTick = useCallback((index: number) => {
		setRovingIndex(index)
		tickRefs.current[index]?.focus()
	}, [])

	const handleKeyDown = useCallback(
		(event: ReactKeyboardEvent<HTMLElement>) => {
			const count = itemsRef.current.length
			if (count === 0) {
				return
			}

			const current = clampNumber(rovingIndex, 0, count - 1)
			switch (event.key) {
				case "ArrowDown":
					event.preventDefault()
					focusTick(Math.min(count - 1, current + 1))
					break
				case "ArrowUp":
					event.preventDefault()
					focusTick(Math.max(0, current - 1))
					break
				case "Home":
					event.preventDefault()
					focusTick(0)
					break
				case "End":
					event.preventDefault()
					focusTick(count - 1)
					break
				case "Enter":
				case " ": {
					event.preventDefault()
					const item = itemsRef.current[current]
					if (item) {
						onSelectRef.current(item.id)
					}
					break
				}
				case "Escape":
					tickRefs.current[current]?.blur()
					break
				default:
					break
			}
		},
		[focusTick, rovingIndex],
	)

	const handleTickFocus = useCallback(
		(index: number) => {
			focusOverrideIndexRef.current = index
			const geometryValue = geometryRef.current
			if (geometryValue) {
				showTooltip(index, geometryValue)
			}
			scheduleFrame()
		},
		[scheduleFrame, showTooltip],
	)

	const handleRailBlur = useCallback(
		(event: ReactFocusEvent<HTMLElement>) => {
			const root = rootRef.current
			if (
				root &&
				event.relatedTarget instanceof Node &&
				root.contains(event.relatedTarget)
			) {
				return
			}
			focusOverrideIndexRef.current = null
			if (latestPointerClientYRef.current === null) {
				applyRest()
			}
		},
		[applyRest],
	)

	const tabStop = clampNumber(rovingIndex, 0, Math.max(0, items.length - 1))

	return (
		<nav
			ref={rootRef}
			aria-label="Message navigation"
			aria-hidden={!visible}
			onKeyDown={handleKeyDown}
			onBlur={handleRailBlur}
			className={cn(
				"sticky top-0 z-20 hidden h-full shrink-0 flex-col justify-center self-start transition-opacity duration-200 sm:flex",
				visible ? "opacity-100" : "pointer-events-none opacity-0",
			)}
			style={{ width: RAIL_WIDTH_PX }}
		>
			<div
				ref={viewportRef}
				onPointerEnter={handlePointerEnter}
				onPointerMove={handlePointerMove}
				onPointerLeave={handlePointerLeave}
				onScroll={handleScroll}
				onClick={handleClick}
				className={cn(
					"relative w-full overflow-y-auto overscroll-contain [contain:layout] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden",
					visible ? "pointer-events-auto" : "pointer-events-none",
				)}
				style={{ maxHeight: `${RAIL_MAX_HEIGHT_RATIO * 100}%` }}
			>
				<div
					className="relative w-full"
					style={{ height: geometry?.contentHeight }}
				>
					{items.map((item, index) => (
						<button
							key={item.id}
							ref={(el) => {
								tickRefs.current[index] = el
							}}
							type="button"
							tabIndex={visible && index === tabStop ? 0 : -1}
							aria-label={`Message ${item.ordinal}: ${item.preview.slice(0, 60)}`}
							aria-describedby={tooltipId}
							aria-current={index === anchorIndex ? "location" : undefined}
							onFocus={() => handleTickFocus(index)}
							className="absolute rounded-full bg-foreground transition-[width,opacity] duration-[90ms] ease-out outline-none focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
							style={{
								left: TICK_LEFT_PAD_PX,
								height: TICK_HEIGHT_PX,
								width: TICK_BASE_W,
								opacity:
									index === anchorIndex
										? TICK_ANCHOR_OPACITY
										: visibleIndexSet.has(index)
											? TICK_VISIBLE_OPACITY
											: TICK_REST_OPACITY,
								willChange: "width, opacity",
							}}
						/>
					))}
				</div>
			</div>
			<div
				ref={tooltipRef}
				role="tooltip"
				id={tooltipId}
				className={cn(
					"pointer-events-none invisible absolute z-30 w-64 -translate-y-1/2 rounded-xl border border-border bg-popover p-2 shadow-lg",
				)}
				style={{ left: RAIL_WIDTH_PX + TOOLTIP_OFFSET_X_PX, top: 0 }}
			>
				<div
					ref={tooltipMessageRef}
					className="line-clamp-2 text-xs leading-snug font-medium text-foreground"
				/>
				<div
					ref={tooltipResponseRef}
					className="mt-1 line-clamp-3 text-xs leading-snug text-muted-foreground"
				/>
			</div>
		</nav>
	)
}