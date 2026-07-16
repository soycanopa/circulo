import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

interface ComposerCommandMenuProps {
	children: ReactNode
	className?: string
}

export function ComposerCommandMenu({ children, className }: ComposerCommandMenuProps) {
	return (
		<div
			className={cn(
				"absolute bottom-full left-0 z-20 mb-2 w-full",
				className,
			)}
		>
			{children}
		</div>
	)
}