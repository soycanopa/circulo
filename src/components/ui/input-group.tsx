import { forwardRef, type ComponentProps, type ReactNode } from "react"
import { cn } from "@/lib/utils"

export function InputGroup({ className, children, ...props }: ComponentProps<"div">) {
	return (
		<div data-slot="input-group" className={cn("flex flex-col", className)} {...props}>
			{children}
		</div>
	)
}

export const InputGroupTextarea = forwardRef<HTMLTextAreaElement, ComponentProps<"textarea">>(
	function InputGroupTextarea({ className, ...props }, ref) {
		return (
			<textarea
				ref={ref}
				className={cn(
					"min-h-16 max-h-48 w-full resize-none bg-transparent px-3 py-2 text-sm outline-none placeholder:text-muted-foreground",
					className,
				)}
				{...props}
			/>
		)
	},
)

export function InputGroupAddon({
	className,
	children,
	align = "block-end",
}: {
	className?: string
	children: ReactNode
	align?: "block-end" | "inline-end"
}) {
	return (
		<div
			data-slot="input-group-addon"
			className={cn(
				"flex items-center gap-1 px-2 py-1.5",
				align === "block-end" && "border-t border-border/50",
				className,
			)}
		>
			{children}
		</div>
	)
}

export function InputGroupButton({
	className,
	variant = "default",
	size = "icon-sm",
	...props
}: ComponentProps<"button"> & { variant?: "default" | "ghost"; size?: "icon-sm" | "sm" }) {
	return (
		<button
			type={props.type ?? "button"}
			className={cn(
				"inline-flex shrink-0 items-center justify-center rounded-md transition-colors disabled:pointer-events-none disabled:opacity-50",
				size === "icon-sm" && "size-7",
				size === "sm" && "h-7 px-2 text-xs",
				variant === "default" && "bg-primary text-primary-foreground hover:bg-primary/90",
				variant === "ghost" && "text-muted-foreground hover:bg-accent hover:text-foreground",
				className,
			)}
			{...props}
		/>
	)
}
