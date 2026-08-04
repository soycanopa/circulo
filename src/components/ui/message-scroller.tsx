import * as React from "react"
import {
	MessageScroller as MessageScrollerPrimitive,
	useMessageScroller,
	useMessageScrollerScrollable,
	useMessageScrollerVisibility,
} from "@shadcn/react/message-scroller"
import { ArrowDown } from "lucide-react"
import { cn } from "@/lib/utils"

function MessageScrollerProvider(
	props: React.ComponentProps<typeof MessageScrollerPrimitive.Provider>,
) {
	return <MessageScrollerPrimitive.Provider {...props} />
}

function MessageScroller({
	className,
	...props
}: React.ComponentProps<typeof MessageScrollerPrimitive.Root>) {
	return (
		<MessageScrollerPrimitive.Root
			data-slot="message-scroller"
			className={cn(
				"group/message-scroller relative flex size-full min-h-0 flex-col overflow-hidden",
				className,
			)}
			{...props}
		/>
	)
}

function MessageScrollerViewport({
	className,
	...props
}: React.ComponentProps<typeof MessageScrollerPrimitive.Viewport>) {
	return (
		<MessageScrollerPrimitive.Viewport
			data-slot="message-scroller-viewport"
			className={cn(
				"size-full min-h-0 min-w-0 overflow-y-auto overscroll-contain",
				className,
			)}
			{...props}
		/>
	)
}

function MessageScrollerContent({
	className,
	...props
}: React.ComponentProps<typeof MessageScrollerPrimitive.Content>) {
	return (
		<MessageScrollerPrimitive.Content
			data-slot="message-scroller-content"
			className={cn("flex h-max min-h-full flex-col", className)}
			{...props}
		/>
	)
}

function MessageScrollerItem({
	className,
	scrollAnchor = false,
	...props
}: React.ComponentProps<typeof MessageScrollerPrimitive.Item>) {
	return (
		<MessageScrollerPrimitive.Item
			data-slot="message-scroller-item"
			scrollAnchor={scrollAnchor}
			className={cn("min-w-0 shrink-0", className)}
			{...props}
		/>
	)
}

function MessageScrollerButton({
	direction = "end",
	className,
	children,
	...props
}: React.ComponentProps<typeof MessageScrollerPrimitive.Button>) {
	return (
		<MessageScrollerPrimitive.Button
			data-slot="message-scroller-button"
			data-direction={direction}
			direction={direction}
			type="button"
			className={cn(
				"absolute left-1/2 z-10 inline-flex size-8 -translate-x-1/2 items-center justify-center rounded-full border border-border bg-surface text-fg shadow-md transition-[translate,scale,opacity] duration-200",
				"hover:bg-white/10",
				"data-[active=false]:pointer-events-none data-[active=false]:scale-95 data-[active=false]:opacity-0",
				"data-[direction=end]:bottom-4 data-[direction=end]:data-[active=false]:translate-y-full",
				"data-[direction=start]:top-4 data-[direction=start]:data-[active=false]:-translate-y-full",
				className,
			)}
			{...props}
		>
			{children ?? (
				<>
					<ArrowDown
						className={cn(
							"size-4",
							direction === "start" && "rotate-180",
						)}
						aria-hidden
					/>
					<span className="sr-only">
						{direction === "end" ? "Scroll to latest" : "Scroll to start"}
					</span>
				</>
			)}
		</MessageScrollerPrimitive.Button>
	)
}

export {
	MessageScrollerProvider,
	MessageScroller,
	MessageScrollerViewport,
	MessageScrollerContent,
	MessageScrollerItem,
	MessageScrollerButton,
	useMessageScroller,
	useMessageScrollerScrollable,
	useMessageScrollerVisibility,
}
