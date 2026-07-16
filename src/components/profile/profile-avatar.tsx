import { cn } from "@/lib/utils"

export function ProfileAvatar({
	initials,
	color,
	className,
	textClassName,
}: {
	initials: string
	color: string
	className?: string
	textClassName?: string
}) {
	return (
		<div
			className={cn(
				"flex shrink-0 items-center justify-center rounded-full font-medium text-white shadow-sm",
				className,
			)}
			style={{ backgroundColor: color }}
		>
			<span className={cn("select-none", textClassName)}>{initials}</span>
		</div>
	)
}