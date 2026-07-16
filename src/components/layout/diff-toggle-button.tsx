import { useAtom, useAtomValue } from "jotai"
import { FileDiff } from "lucide-react"
import { DiffStatLabel } from "@/components/chat/diff-stat-label"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { collectSessionDiffStats } from "@/lib/session-diff-stats"
import { collectSessionDiffs } from "@/lib/session-diffs"
import { cn } from "@/lib/utils"
import { diffPanelOpenAtom, messagesAtom } from "@/stores/atoms"
import { useMemo } from "react"

const chipClassName =
	"pointer-events-auto inline-flex h-7 shrink-0 items-center gap-1.5 rounded-lg border-0 px-1.5 pr-2.5 text-[11px] font-normal transition-colors text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"

export function DiffToggleButton() {
	const [diffPanelOpen, setDiffPanelOpen] = useAtom(diffPanelOpenAtom)
	const messages = useAtomValue(messagesAtom)

	const entries = useMemo(() => collectSessionDiffs(messages), [messages])
	const stats = useMemo(() => collectSessionDiffStats(messages), [messages])

	if (entries.length === 0 && !diffPanelOpen) return null

	return (
		<button
			type="button"
			{...windowNoDragProps()}
			onClick={() => setDiffPanelOpen((open) => !open)}
			title={diffPanelOpen ? "Ocultar cambios (⌘⇧D)" : "Mostrar cambios (⌘⇧D)"}
			aria-pressed={diffPanelOpen}
			data-pressed={diffPanelOpen ? "" : undefined}
			className={cn(chipClassName, diffPanelOpen && "bg-sidebar-accent text-sidebar-accent-foreground")}
		>
			<FileDiff className="size-3.5 shrink-0 opacity-70" aria-hidden />
			<DiffStatLabel additions={stats.additions} deletions={stats.deletions} />
		</button>
	)
}