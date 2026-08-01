import type { PointerEvent as ReactPointerEvent } from "react"
import { cn } from "@/lib/utils"

interface ResizeHandleProps {
	onPointerDown: (event: ReactPointerEvent<HTMLDivElement>) => void
	className?: string
}

export function ResizeHandle({ onPointerDown, className }: ResizeHandleProps) {
	return (
		<div
			role="separator"
			aria-orientation="vertical"
			aria-label="Resize panel"
			onPointerDown={onPointerDown}
			className={cn(
				"relative z-20 w-1 shrink-0 cursor-col-resize touch-none",
				"before:absolute before:inset-y-0 before:-left-1 before:w-2 before:content-['']",
				"bg-transparent transition-colors hover:bg-white/10 active:bg-white/15",
				className,
			)}
		/>
	)
}
