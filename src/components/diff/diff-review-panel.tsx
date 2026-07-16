import { useAtomValue } from "jotai"
import { FileDiff, X } from "lucide-react"
import { useEffect, useMemo, useRef } from "react"
import { DiffStatLabel } from "@/components/chat/diff-stat-label"
import { CollapsedDiffRow } from "@/components/tools/collapsed-diff-row"
import { useDiffPanel } from "@/hooks/use-diff-panel"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { collectSessionDiffStats } from "@/lib/session-diff-stats"
import { collectSessionDiffs, type SessionDiffEntry } from "@/lib/session-diffs"
import { APP_BAR_HEIGHT } from "@/lib/window-chrome"
import { activeDiffToolIdAtom, messagesAtom } from "@/stores/atoms"
import type { ToolCallState } from "@/types/acp"

function entryToToolCall(entry: SessionDiffEntry): ToolCallState {
	return {
		id: entry.id,
		title: entry.title,
		status: "completed",
		content: "",
		diff: {
			path: entry.path,
			oldText: entry.oldText,
			newText: entry.newText,
		},
	}
}

export function DiffReviewPanel() {
	const messages = useAtomValue(messagesAtom)
	const activeDiffToolId = useAtomValue(activeDiffToolIdAtom)
	const { closeDiffPanel } = useDiffPanel()
	const listRef = useRef<HTMLDivElement>(null)

	const entries = useMemo(() => collectSessionDiffs(messages), [messages])
	const stats = useMemo(() => collectSessionDiffStats(messages), [messages])

	useEffect(() => {
		if (!activeDiffToolId || !listRef.current) return
		const target = listRef.current.querySelector(
			`[data-diff-entry-id="${activeDiffToolId}"]`,
		)
		if (target instanceof HTMLElement) {
			target.scrollIntoView({ behavior: "smooth", block: "nearest" })
		}
	}, [activeDiffToolId, entries.length])

	return (
		<aside
			data-slot="diff-review-panel"
			className="flex h-full min-h-0 min-w-0 flex-col overflow-hidden border-l border-border/50 bg-background"
			{...windowNoDragProps()}
		>
			<header
				className="relative z-[52] box-border flex shrink-0 items-center gap-2 border-b border-border/50 px-3"
				style={{ height: APP_BAR_HEIGHT }}
			>
				<FileDiff className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />
				<span className="min-w-0 flex-1 truncate text-[11px] font-medium text-foreground">
					Archivos modificados
					{entries.length > 0 ? (
						<span className="ml-1.5 text-muted-foreground">({entries.length})</span>
					) : null}
				</span>
				<DiffStatLabel
					additions={stats.additions}
					deletions={stats.deletions}
					className="shrink-0 text-[11px]"
				/>
				<button
					type="button"
					onClick={(event) => {
						event.stopPropagation()
						closeDiffPanel()
					}}
					className="flex size-7 shrink-0 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-sidebar-accent-hover hover:text-foreground"
					aria-label="Cerrar panel de cambios"
					title="Cerrar panel (⌘⇧D)"
				>
					<X className="size-3.5" />
				</button>
			</header>

			{entries.length === 0 ? (
				<div className="flex min-h-0 flex-1 items-center justify-center px-6 text-center text-sm text-muted-foreground">
					No hay archivos modificados en esta sesión.
				</div>
			) : (
				<div
					ref={listRef}
					className="scrollbar-thin min-h-0 flex-1 space-y-1.5 overflow-y-auto p-2"
				>
					{entries.map((entry) => (
						<div key={entry.id} data-diff-entry-id={entry.id}>
							<CollapsedDiffRow
								toolCall={entryToToolCall(entry)}
								showPanelAction={false}
							/>
						</div>
					))}
				</div>
			)}
		</aside>
	)
}