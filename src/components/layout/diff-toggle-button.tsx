import { useAtom, useAtomValue } from "jotai"
import { FileDiff } from "lucide-react"
import { useMemo } from "react"
import { DiffStatLabel } from "@/components/chat/diff-stat-label"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { collectSessionDiffStats } from "@/lib/session-diff-stats"
import { cn } from "@/lib/utils"
import { diffPanelOpenAtom, messagesAtom } from "@/stores/atoms"

const controlButtonClass =
	"pointer-events-auto flex h-7 shrink-0 items-center justify-center rounded-md text-sidebar-foreground/70 transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"

export function DiffToggleButton() {
	const [diffPanelOpen, setDiffPanelOpen] = useAtom(diffPanelOpenAtom)
	const messages = useAtomValue(messagesAtom)

	const stats = useMemo(() => collectSessionDiffStats(messages), [messages])
	const hasStats = stats.additions > 0 || stats.deletions > 0

	return (
		<button
			type="button"
			{...windowNoDragProps()}
			onClick={() => setDiffPanelOpen((open) => !open)}
			title={diffPanelOpen ? "Ocultar cambios (⌘⇧D)" : "Mostrar cambios (⌘⇧D)"}
			aria-pressed={diffPanelOpen}
			className={cn(
				controlButtonClass,
				hasStats ? "gap-1.5 px-1.5 pr-2.5 text-[11px] font-normal" : "size-7",
				diffPanelOpen && "bg-sidebar-accent text-sidebar-accent-foreground",
			)}
		>
			<FileDiff className="size-3.5 shrink-0" aria-hidden />
			{hasStats ? (
				<DiffStatLabel additions={stats.additions} deletions={stats.deletions} />
			) : null}
		</button>
	)
}