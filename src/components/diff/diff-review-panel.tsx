import { useAtom, useAtomValue } from "jotai"
import { Expand, FileDiff, X } from "lucide-react"
import { useEffect, useMemo } from "react"
import { DiffFileList } from "@/components/diff/diff-file-list"
import { PierreFileDiff } from "@/components/diff/pierre-diff-view"
import { useDiffPanel } from "@/hooks/use-diff-panel"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { collectSessionDiffs } from "@/lib/session-diffs"
import { activeDiffToolIdAtom, messagesAtom } from "@/stores/atoms"
import type { ChatMessage, ToolCallState } from "@/types/acp"

function findToolCallById(messages: ChatMessage[], toolCallId: string): ToolCallState | null {
	for (const message of messages) {
		if (message.role !== "assistant") continue
		const tool = message.toolCalls.find((entry) => entry.id === toolCallId)
		if (tool) return tool
	}
	return null
}

export function DiffReviewPanel() {
	const messages = useAtomValue(messagesAtom)
	const [activeDiffToolId, setActiveDiffToolId] = useAtom(activeDiffToolIdAtom)
	const { closeDiffPanel, openDiffFullscreen } = useDiffPanel()

	const entries = useMemo(() => collectSessionDiffs(messages), [messages])
	const active =
		entries.find((entry) => entry.id === activeDiffToolId) ?? entries[entries.length - 1]

	useEffect(() => {
		if (entries.length === 0) return
		if (!activeDiffToolId || !entries.some((entry) => entry.id === activeDiffToolId)) {
			setActiveDiffToolId(entries[entries.length - 1].id)
		}
	}, [entries, activeDiffToolId, setActiveDiffToolId])

	return (
		<aside
			data-slot="diff-review-panel"
			className="flex h-full min-h-0 flex-col overflow-hidden border-l border-border bg-card"
			{...windowNoDragProps()}
		>
			<header className="flex h-7 shrink-0 items-center gap-2 border-b border-border/60 px-2">
				<FileDiff className="size-3 shrink-0 text-muted-foreground" />
				<span className="min-w-0 flex-1 truncate text-[11px] font-medium text-foreground">
					Cambios
					{entries.length > 0 ? (
						<span className="ml-1 text-muted-foreground">({entries.length})</span>
					) : null}
				</span>
				{active ? (
					<button
						type="button"
						onClick={() => {
							const toolCall = findToolCallById(messages, active.id)
							if (toolCall) openDiffFullscreen(toolCall)
						}}
						className="flex size-5 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-[rgba(255,255,255,0.06)] hover:text-foreground"
						title="Pantalla completa"
						aria-label="Abrir diff en pantalla completa"
					>
						<Expand className="size-3" />
					</button>
				) : null}
				<button
					type="button"
					onClick={closeDiffPanel}
					className="flex size-5 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-[rgba(255,255,255,0.06)] hover:text-foreground"
					aria-label="Cerrar panel de cambios"
					title="Cerrar panel (⌘⇧D)"
				>
					<X className="size-3" />
				</button>
			</header>

			<div className="grid min-h-0 flex-1 grid-cols-[minmax(9.5rem,34%)_minmax(0,1fr)]">
				<div className="min-h-0 border-r border-border/60 bg-background/40">
					<DiffFileList
						entries={entries}
						activeId={active?.id ?? ""}
						onSelect={setActiveDiffToolId}
					/>
				</div>

				<div className="relative min-h-0 min-w-0 bg-[#141414]">
					{active ? (
						<PierreFileDiff
							path={active.path}
							oldText={active.oldText}
							newText={active.newText}
							fill
							className="h-full rounded-none border-0"
						/>
					) : (
						<div className="flex h-full items-center justify-center px-4 text-center text-sm text-muted-foreground">
							Los diffs de archivos editados aparecerán aquí.
						</div>
					)}
				</div>
			</div>
		</aside>
	)
}