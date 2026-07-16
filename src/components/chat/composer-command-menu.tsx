import type { ReactNode } from "react"
import { Command, CommandInput } from "@/components/ui/command"
import { cn } from "@/lib/utils"

interface ComposerCommandMenuProps {
	children: ReactNode
	className?: string
	value?: string
	onValueChange?: (value: string) => void
	query?: string
	placeholder?: string
}

export function ComposerCommandMenu({
	children,
	className,
	value,
	onValueChange,
	query = "",
	placeholder = "Buscar…",
}: ComposerCommandMenuProps) {
	return (
		<div
			className={cn(
				"absolute bottom-full left-0 z-20 mb-2 w-full",
				className,
			)}
		>
			<Command
				shouldFilter={false}
				loop
				value={value}
				onValueChange={onValueChange}
				className="overflow-hidden rounded-lg border border-popover-border shadow-md"
			>
				<CommandInput
					value={query}
					readOnly
					tabIndex={-1}
					placeholder={placeholder}
					aria-hidden
					className="pointer-events-none"
					wrapperClassName="pointer-events-none"
				/>
				{children}
			</Command>
		</div>
	)
}