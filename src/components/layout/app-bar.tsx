import { windowDragRegionProps } from "@/hooks/use-window-drag"
import { APP_BAR_HEIGHT } from "@/lib/window-chrome"
import { cn } from "@/lib/utils"

interface AppBarProps {
	className?: string
}

/** Chrome spacer — symmetric padding with border at the bottom edge. */
export function AppBar({ className }: AppBarProps) {
	return (
		<div
			data-slot="app-bar"
			{...windowDragRegionProps()}
			className={cn("relative z-[45] box-border shrink-0 border-b border-border/50", className)}
			style={{ height: APP_BAR_HEIGHT }}
		/>
	)
}