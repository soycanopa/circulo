import { useAtom } from "jotai"
import { FileDiff } from "lucide-react"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { cn } from "@/lib/utils"
import { diffPanelOpenAtom } from "@/stores/atoms"

const controlButtonClass =
	"pointer-events-auto flex size-7 shrink-0 items-center justify-center rounded-md text-sidebar-foreground/70 transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"

export function DiffToggleButton() {
	const [diffPanelOpen, setDiffPanelOpen] = useAtom(diffPanelOpenAtom)

	return (
		<button
			type="button"
			{...windowNoDragProps()}
			onClick={() => setDiffPanelOpen((open) => !open)}
			title={diffPanelOpen ? "Ocultar cambios (⌘⇧D)" : "Mostrar cambios (⌘⇧D)"}
			aria-pressed={diffPanelOpen}
			className={cn(
				controlButtonClass,
				diffPanelOpen && "bg-sidebar-accent text-sidebar-accent-foreground",
			)}
		>
			<FileDiff className="size-3.5" />
		</button>
	)
}