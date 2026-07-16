import { useAtom, useAtomValue } from "jotai"
import { useMemo } from "react"
import { DiffPanelOpenButton } from "@/components/tools/diff-panel-open-button"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { cn } from "@/lib/utils"
import { collectSessionDiffStats } from "@/lib/session-diff-stats"
import { diffPanelOpenAtom, messagesAtom } from "@/stores/atoms"

export function DiffToggleButton() {
	const [diffPanelOpen, setDiffPanelOpen] = useAtom(diffPanelOpenAtom)
	const messages = useAtomValue(messagesAtom)
	const stats = useMemo(() => collectSessionDiffStats(messages), [messages])

	return (
		<DiffPanelOpenButton
			{...windowNoDragProps()}
			onClick={() => setDiffPanelOpen((open) => !open)}
			title={diffPanelOpen ? "Ocultar cambios (⌘⇧D)" : "Mostrar cambios (⌘⇧D)"}
			ariaLabel={diffPanelOpen ? "Ocultar panel de cambios" : "Mostrar panel de cambios"}
			active={diffPanelOpen}
			activeClassName="bg-sidebar-accent text-sidebar-accent-foreground"
			stats={stats}
			className={cn(
				"pointer-events-auto text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
			)}
		/>
	)
}