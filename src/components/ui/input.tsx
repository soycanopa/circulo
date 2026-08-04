import * as React from "react"
import { cn } from "@/lib/utils"

const Input = React.forwardRef<HTMLInputElement, React.ComponentProps<"input">>(
	({ className, type, ...props }, ref) => (
		<input
			type={type}
			ref={ref}
			className={cn(
				"flex h-8 w-full rounded-md border border-border bg-black/30 px-2.5 py-1.5 text-xs text-fg shadow-sm transition-colors",
				"placeholder:text-muted focus-visible:border-white/20 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-white/10",
				"disabled:cursor-not-allowed disabled:opacity-50",
				className,
			)}
			{...props}
		/>
	),
)
Input.displayName = "Input"

export { Input }
