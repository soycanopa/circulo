import { cn } from "@/lib/utils"

export function ProfileAvatar({
	initials,
	color,
	image,
	className,
	textClassName,
}: {
	initials: string
	color: string
	image?: string | null
	className?: string
	textClassName?: string
}) {
	return (
		<div
			className={cn(
				"flex shrink-0 items-center justify-center overflow-hidden rounded-full text-white shadow-sm",
				className,
			)}
			style={image ? undefined : { backgroundColor: color }}
		>
			{image ? (
				<img
					src={image}
					alt=""
					draggable={false}
					className="size-full object-cover"
				/>
			) : (
				<span className={cn("select-none font-semibold tracking-tight", textClassName)}>
					{initials}
				</span>
			)}
		</div>
	)
}