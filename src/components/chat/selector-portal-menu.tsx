import {
	useEffect,
	useLayoutEffect,
	useRef,
	useState,
	type ReactNode,
	type RefObject,
} from "react"
import { createPortal } from "react-dom"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { cn } from "@/lib/utils"

const VIEWPORT_PADDING = 8
const ANCHOR_GAP = 8
const ESTIMATED_MENU_HEIGHT = 200

interface SelectorPortalMenuProps {
	open: boolean
	anchorRef: RefObject<HTMLElement | null>
	onClose: () => void
	children: ReactNode
	className?: string
	minWidth?: number
	/** Bias placement when both sides fit. Context controls at the bottom should use "above". */
	preferPlacement?: "above" | "below" | "auto"
}

export function SelectorPortalMenu({
	open,
	anchorRef,
	onClose,
	children,
	className,
	minWidth = 176,
	preferPlacement = "auto",
}: SelectorPortalMenuProps) {
	const menuRef = useRef<HTMLDivElement>(null)
	const [position, setPosition] = useState<{
		top: number
		left: number
		placement: "above" | "below"
		maxWidth: number
	} | null>(null)
	const noDragProps = windowNoDragProps()

	function updatePosition() {
		const rect = anchorRef.current?.getBoundingClientRect()
		if (!rect) return

		const viewportWidth = window.innerWidth
		const viewportHeight = window.innerHeight
		const maxWidth = Math.max(120, viewportWidth - VIEWPORT_PADDING * 2)
		const menuWidth = Math.min(menuRef.current?.offsetWidth ?? minWidth, maxWidth)
		const menuHeight = menuRef.current?.offsetHeight || ESTIMATED_MENU_HEIGHT

		const spaceAbove = rect.top - VIEWPORT_PADDING
		const spaceBelow = viewportHeight - rect.bottom - VIEWPORT_PADDING
		const fitsAbove = spaceAbove >= menuHeight + ANCHOR_GAP
		const fitsBelow = spaceBelow >= menuHeight + ANCHOR_GAP

		let placement: "above" | "below"
		if (fitsAbove && !fitsBelow) {
			placement = "above"
		} else if (fitsBelow && !fitsAbove) {
			placement = "below"
		} else if (fitsAbove && fitsBelow) {
			if (preferPlacement === "above" || preferPlacement === "below") {
				placement = preferPlacement
			} else {
				placement = rect.top > viewportHeight * 0.55 ? "above" : "below"
			}
		} else {
			placement = spaceAbove >= spaceBelow ? "above" : "below"
		}

		let top: number
		if (placement === "below") {
			top = rect.bottom + ANCHOR_GAP
			const maxTop = viewportHeight - VIEWPORT_PADDING - menuHeight
			top = Math.min(top, maxTop)
			top = Math.max(VIEWPORT_PADDING, top)
		} else {
			top = rect.top - ANCHOR_GAP
			const minTop = VIEWPORT_PADDING + menuHeight
			if (top < minTop) top = minTop
		}

		let left = rect.left
		if (left + menuWidth > viewportWidth - VIEWPORT_PADDING) {
			left = viewportWidth - VIEWPORT_PADDING - menuWidth
		}
		left = Math.max(VIEWPORT_PADDING, left)

		setPosition({ top, left, placement, maxWidth })
	}

	useEffect(() => {
		if (!open) {
			setPosition(null)
			return
		}

		updatePosition()

		function handlePointerDown(event: PointerEvent) {
			const target = event.target
			if (!(target instanceof Node)) return
			if (anchorRef.current?.contains(target)) return
			if (target instanceof Element && target.closest("[data-selector-portal-menu]")) return
			onClose()
		}

		function handleLayoutChange() {
			updatePosition()
		}

		document.addEventListener("pointerdown", handlePointerDown, true)
		window.addEventListener("resize", handleLayoutChange)
		window.addEventListener("scroll", handleLayoutChange, true)
		return () => {
			document.removeEventListener("pointerdown", handlePointerDown, true)
			window.removeEventListener("resize", handleLayoutChange)
			window.removeEventListener("scroll", handleLayoutChange, true)
		}
	}, [open, anchorRef, onClose, minWidth, preferPlacement])

	useLayoutEffect(() => {
		if (!open) return
		updatePosition()
	}, [open, children, minWidth, preferPlacement])

	if (!open || !position) return null

	return createPortal(
		<div
			ref={menuRef}
			data-selector-portal-menu
			{...noDragProps}
			className={cn(
				"fixed z-[200] overflow-x-hidden overflow-y-auto rounded-lg border border-popover-border bg-popover shadow-lg",
				position.placement === "above" && "-translate-y-full",
				className,
			)}
			style={{
				top: position.top,
				left: position.left,
				minWidth: Math.min(minWidth, position.maxWidth),
				maxWidth: position.maxWidth,
				maxHeight: `calc(100vh - ${VIEWPORT_PADDING * 2}px)`,
			}}
		>
			{children}
		</div>,
		document.body,
	)
}