import { useEffect, useState, type ReactNode, type RefObject } from "react"
import { createPortal } from "react-dom"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { cn } from "@/lib/utils"

interface SelectorPortalMenuProps {
	open: boolean
	anchorRef: RefObject<HTMLElement | null>
	onClose: () => void
	children: ReactNode
	className?: string
	minWidth?: number
}

export function SelectorPortalMenu({
	open,
	anchorRef,
	onClose,
	children,
	className,
	minWidth = 176,
}: SelectorPortalMenuProps) {
	const [position, setPosition] = useState<{
		top: number
		left: number
		placement: "above" | "below"
	} | null>(null)
	const noDragProps = windowNoDragProps()

	function updatePosition() {
		const rect = anchorRef.current?.getBoundingClientRect()
		if (!rect) return
		const spaceAbove = rect.top
		const spaceBelow = window.innerHeight - rect.bottom
		const placement = spaceAbove >= spaceBelow ? "above" : "below"
		setPosition({
			top: placement === "above" ? rect.top - 8 : rect.bottom + 8,
			left: rect.left,
			placement,
		})
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
	}, [open, anchorRef, onClose])

	if (!open || !position) return null

	return createPortal(
		<div
			data-selector-portal-menu
			{...noDragProps}
			className={cn(
				"fixed z-[200] overflow-hidden rounded-lg border border-border bg-popover shadow-lg",
				position.placement === "above" && "-translate-y-full",
				className,
			)}
			style={{ top: position.top, left: position.left, minWidth }}
		>
			{children}
		</div>,
		document.body,
	)
}