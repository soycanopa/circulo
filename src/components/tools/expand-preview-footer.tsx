import { ChevronDown, ChevronUp } from "lucide-react"
import { cn } from "@/lib/utils"

interface ExpandPreviewFooterProps {
	label: string
	onClick: () => void
	variant?: "expand" | "collapse"
	className?: string
}

export function ExpandPreviewFooter({
	label,
	onClick,
	variant = "expand",
	className,
}: ExpandPreviewFooterProps) {
	const Icon = variant === "collapse" ? ChevronUp : ChevronDown

	return (
		<button
			type="button"
			onClick={onClick}
			className={cn(
				"group flex w-full items-center justify-center gap-1 rounded-md py-1.5 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-accent/30 hover:text-foreground",
				className,
			)}
		>
			<Icon className="size-3 opacity-60 transition-transform group-hover:opacity-100" />
			{label}
		</button>
	)
}