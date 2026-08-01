import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

interface AppShellProps {
	sidebar?: ReactNode
	children: ReactNode
	panel?: ReactNode
	className?: string
}

/** Minimal Palot-inspired desktop shell: sidebar + content. */
export function AppShell({ sidebar, children, panel, className }: AppShellProps) {
	return (
		<div className={cn("flex h-full min-h-0 w-full overflow-hidden bg-content", className)}>
			<aside className="flex w-72 shrink-0 flex-col border-r border-border bg-sidebar">
				{sidebar}
			</aside>
			<main className="flex min-w-0 flex-1 flex-col bg-content">{children}</main>
			{panel}
		</div>
	)
}
