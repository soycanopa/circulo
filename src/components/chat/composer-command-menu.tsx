import type { ReactNode } from "react"
import { Command } from "@/components/ui/command"
import { cn } from "@/lib/utils"

interface ComposerCommandMenuProps {
	children: ReactNode
	className?: string
	value?: string
	onValueChange?: (value: string) => void
}

export function ComposerCommandMenu({
	children,
	className,
	value,
	onValueChange,
}: ComposerCommandMenuProps) {
	return (
		<div
			className={cn(
				"absolute bottom-full left-0 z-20 mb-2 w-full overflow-hidden rounded-lg border border-popover-border bg-popover shadow-lg",
				className,
			)}
		>
			<Command
				shouldFilter={false}
				loop
				value={value}
				onValueChange={onValueChange}
			>
				{children}
			</Command>
		</div>
	)
}