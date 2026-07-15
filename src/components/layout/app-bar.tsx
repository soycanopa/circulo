import type { ReactNode } from "react"
import { APP_BAR_HEIGHT, isTauri } from "@/lib/window-chrome"
import { cn } from "@/lib/utils"

interface AppBarProps {
	sidebarCollapsed: boolean
	children?: ReactNode
	className?: string
}

export function AppBar({ sidebarCollapsed, children, className }: AppBarProps) {
	return (
		<div
			data-slot="app-bar"
			data-tauri-drag-region={isTauri ? true : undefined}
			className={cn(
				"relative z-[45] flex shrink-0 items-center border-b border-border/50 pl-4 pr-3 transition-[padding-left] duration-250 ease-in-out",
				sidebarCollapsed &&
					"pl-[calc(var(--window-controls-inset)-var(--shell-inset))]",
				className,
			)}
			style={{ height: APP_BAR_HEIGHT }}
		>
			<div
				data-tauri-drag-region={isTauri ? true : undefined}
				className="relative flex h-full min-w-0 flex-1 items-center"
			>
				{children}
			</div>
		</div>
	)
}