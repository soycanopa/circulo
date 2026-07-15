import { cn } from "@/lib/utils"

export const SELECTOR_ITEM_CLASS =
	"flex w-full items-center rounded-md px-2 py-1.5 text-left text-xs transition-colors hover:bg-white/10 hover:text-foreground"

export const SELECTOR_ITEM_ACTIVE_CLASS = "bg-white/12 text-foreground"

interface SelectorMenuItemProps {
	active?: boolean
	onClick: () => void
	children: React.ReactNode
	className?: string
}

export function SelectorMenuItem({
	active,
	onClick,
	children,
	className,
}: SelectorMenuItemProps) {
	return (
		<button
			type="button"
			className={cn(SELECTOR_ITEM_CLASS, active && SELECTOR_ITEM_ACTIVE_CLASS, className)}
			onClick={onClick}
		>
			{children}
		</button>
	)
}