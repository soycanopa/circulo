import { PanelLeft, Plus } from "lucide-react"
import {
	SHELL_INSET,
	WINDOW_CONTROLS_LEFT,
	WINDOW_CONTROL_TOP,
} from "@/lib/window-chrome"
import { cn } from "@/lib/utils"

interface WindowControlsProps {
	sidebarOpen: boolean
	onToggleSidebar: () => void
	onNewThread: () => void
	className?: string
}

const controlButtonClass =
	"pointer-events-auto flex size-7 shrink-0 items-center justify-center rounded-md text-sidebar-foreground/70 transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"

export function WindowControls({
	sidebarOpen,
	onToggleSidebar,
	onNewThread,
	className,
}: WindowControlsProps) {
	return (
		<div
			className={cn("pointer-events-none absolute z-50 flex items-center gap-0.5", className)}
			style={{
				top: WINDOW_CONTROL_TOP - SHELL_INSET,
				left: WINDOW_CONTROLS_LEFT - SHELL_INSET,
			}}
		>
			<button
				type="button"
				onClick={onToggleSidebar}
				title={sidebarOpen ? "Ocultar sidebar (⌘B)" : "Mostrar sidebar (⌘B)"}
				className={controlButtonClass}
			>
				<PanelLeft className="size-3.5" />
			</button>
			{!sidebarOpen ? (
				<button
					type="button"
					onClick={onNewThread}
					title="New Thread"
					className={controlButtonClass}
				>
					<Plus className="size-3.5" />
				</button>
			) : null}
		</div>
	)
}