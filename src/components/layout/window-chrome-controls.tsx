import { PanelLeft, PanelLeftClose } from "lucide-react"
import { TRAFFIC_LIGHT_GUTTER } from "@/components/layout/app-shell"
import { cn } from "@/lib/utils"

/** Inset between macOS traffic lights and sidebar controls. */
const TRAFFIC_LIGHT_CONTENT_INSET = "pl-1"

interface WindowChromeControlsProps {
	sidebarOpen: boolean
	onToggleSidebar: () => void
	layout?: "sidebar" | "inline"
	className?: string
}

export function WindowChromeControls({
	sidebarOpen,
	onToggleSidebar,
	layout = "sidebar",
	className,
}: WindowChromeControlsProps) {
	const ToggleIcon = sidebarOpen ? PanelLeftClose : PanelLeft
	const toggleTitle = sidebarOpen ? "Hide sidebar" : "Show sidebar"

	return (
		<>
			<div className={TRAFFIC_LIGHT_GUTTER} aria-hidden />
			<div
				className={cn(
					"flex h-12 items-center",
					TRAFFIC_LIGHT_CONTENT_INSET,
					layout === "sidebar" ? "min-w-0 flex-1 pr-3" : "shrink-0 pr-2",
					className,
				)}
			>
				{layout === "sidebar" ? <div className="min-w-0 flex-1" /> : null}
				<button
					type="button"
					onClick={onToggleSidebar}
					className="inline-flex size-6 shrink-0 items-center justify-center rounded text-muted transition hover:bg-white/5 hover:text-fg"
					title={toggleTitle}
					data-tauri-drag-region="false"
				>
					<ToggleIcon className="size-4" />
				</button>
			</div>
		</>
	)
}
