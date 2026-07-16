import { useAtom } from "jotai"
import { TerminalSquare } from "lucide-react"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { cn } from "@/lib/utils"
import { terminalOpenAtom } from "@/stores/atoms"

const controlButtonClass =
	"pointer-events-auto flex size-7 shrink-0 items-center justify-center rounded-md text-sidebar-foreground/70 transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"

export function TerminalToggleButton() {
	const [terminalOpen, setTerminalOpen] = useAtom(terminalOpenAtom)

	return (
		<button
			type="button"
			{...windowNoDragProps()}
			onClick={() => setTerminalOpen((open) => !open)}
			title={terminalOpen ? "Ocultar terminal (⌘J)" : "Mostrar terminal (⌘J)"}
			aria-pressed={terminalOpen}
			className={cn(
				controlButtonClass,
				terminalOpen && "bg-sidebar-accent text-sidebar-accent-foreground",
			)}
		>
			<TerminalSquare className="size-3.5" />
		</button>
	)
}