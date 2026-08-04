import * as React from "react"
import { cn } from "@/lib/utils"

export interface SwitchProps
	extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, "onChange"> {
	checked: boolean
	onCheckedChange: (checked: boolean) => void
}

export const Switch = React.forwardRef<HTMLButtonElement, SwitchProps>(
	({ checked, onCheckedChange, disabled, className, ...props }, ref) => (
		<button
			ref={ref}
			type="button"
			role="switch"
			aria-checked={checked}
			disabled={disabled}
			onClick={() => onCheckedChange(!checked)}
			className={cn(
				"relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors",
				"focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-white/25",
				checked ? "bg-emerald-500/75" : "bg-white/15",
				disabled && "cursor-not-allowed opacity-50",
				className,
			)}
			{...props}
		>
			<span
				className={cn(
					"pointer-events-none block size-4 rounded-full bg-white shadow-sm transition-transform",
					checked ? "translate-x-[18px]" : "translate-x-0.5",
				)}
			/>
		</button>
	),
)
Switch.displayName = "Switch"
