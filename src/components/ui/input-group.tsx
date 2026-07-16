import {
	forwardRef,
	useCallback,
	useLayoutEffect,
	useRef,
	useState,
	type ComponentProps,
	type ReactNode,
} from "react"
import { cn } from "@/lib/utils"

const TEXTAREA_LINE_HEIGHT_REM = 1.375
const TEXTAREA_VERTICAL_PADDING_REM = 1

function textareaHeightForLines(lines: number) {
	return `calc(${TEXTAREA_LINE_HEIGHT_REM * lines}rem + ${TEXTAREA_VERTICAL_PADDING_REM}rem)`
}

type InputGroupTextareaProps = ComponentProps<"textarea"> & {
	maxRows?: number
}

export function InputGroup({ className, children, ...props }: ComponentProps<"div">) {
	return (
		<div data-slot="input-group" className={cn("flex flex-col", className)} {...props}>
			{children}
		</div>
	)
}

export const InputGroupTextarea = forwardRef<HTMLTextAreaElement, InputGroupTextareaProps>(
	function InputGroupTextarea({ className, maxRows = 8, onChange, value, ...props }, ref) {
		const textareaRef = useRef<HTMLTextAreaElement | null>(null)
		const [scrollable, setScrollable] = useState(false)

		const resizeToContent = useCallback(
			(element: HTMLTextAreaElement | null) => {
				if (!element) return
				const maxHeightPx = maxRows * (TEXTAREA_LINE_HEIGHT_REM * 16) + TEXTAREA_VERTICAL_PADDING_REM * 16
				element.style.height = "auto"
				const nextHeight = Math.min(element.scrollHeight, maxHeightPx)
				element.style.height = `${nextHeight}px`
				setScrollable(element.scrollHeight > maxHeightPx)
			},
			[maxRows],
		)

		useLayoutEffect(() => {
			resizeToContent(textareaRef.current)
		}, [resizeToContent, value])

		return (
			<div className={cn(scrollable && "pr-2")}>
				<textarea
					data-slot="chat-input-textarea"
					ref={(element) => {
						textareaRef.current = element
						resizeToContent(element)
						if (typeof ref === "function") ref(element)
						else if (ref) ref.current = element
					}}
					rows={1}
					value={value}
					className={cn(
						"scrollbar-thin w-full resize-none bg-transparent py-2 pl-3 text-sm leading-[1.375rem] outline-none placeholder:text-muted-foreground",
						scrollable ? "overflow-y-auto pr-1" : "overflow-hidden pr-3",
						className,
					)}
					style={{
						minHeight: textareaHeightForLines(1),
						maxHeight: textareaHeightForLines(maxRows),
					}}
					onChange={(event) => {
						resizeToContent(event.currentTarget)
						onChange?.(event)
					}}
					{...props}
				/>
			</div>
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
				align === "block-end" && "input-group-addon-divider",
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
				size === "sm" && "h-8 px-2.5 text-sm",
				variant === "default" && "bg-primary text-primary-foreground hover:bg-primary/90",
				variant === "ghost" && "text-muted-foreground hover:bg-accent hover:text-foreground",
				className,
			)}
			{...props}
		/>
	)
}
