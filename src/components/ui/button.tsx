import type { ButtonHTMLAttributes } from "react"
import { cn } from "@/lib/utils"

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
	variant?: "default" | "secondary" | "destructive" | "ghost"
	size?: "default" | "sm"
}

export function Button({
	className,
	variant = "default",
	size = "default",
	...props
}: ButtonProps) {
	return (
		<button
			className={cn(
				"inline-flex items-center justify-center rounded-md font-medium transition-colors disabled:pointer-events-none disabled:opacity-50",
				size === "default" && "h-9 px-4 text-sm",
				size === "sm" && "h-8 px-3 text-xs",
				variant === "default" && "bg-primary text-primary-foreground hover:opacity-90",
				variant === "secondary" &&
					"bg-secondary text-secondary-foreground hover:bg-accent",
				variant === "destructive" &&
					"bg-destructive text-white hover:opacity-90",
				variant === "ghost" && "hover:bg-accent hover:text-accent-foreground",
				className,
			)}
			{...props}
		/>
	)
}