import AnsiToHtml from "ansi-to-react"
import { useMemo, useState } from "react"
import { CodeBlock } from "@/components/chat/code-block"
import { PierreFileDiff } from "@/components/diff/pierre-diff-view"
import { OverlayShell } from "@/components/overlays/overlay-shell"
import { useToolOverlay } from "@/hooks/use-tool-overlay"
import {
	buildToolPreview,
	collectDiffTools,
	type MultiDiffEntry,
} from "@/lib/tool-preview"
import { cn } from "@/lib/utils"

function DiffTabs({
	entries,
	activeId,
	onSelect,
}: {
	entries: MultiDiffEntry[]
	activeId: string
	onSelect: (id: string) => void
}) {
	return (
		<div className="flex flex-wrap gap-1 border-b border-border/60 px-3 py-2">
			{entries.map((entry) => (
				<button
					key={entry.id}
					type="button"
					onClick={() => onSelect(entry.id)}
					className={cn(
						"max-w-xs truncate rounded-md px-2.5 py-1 font-mono text-xs transition-colors",
						activeId === entry.id
							? "bg-accent text-foreground"
							: "text-muted-foreground hover:bg-accent/60 hover:text-foreground",
					)}
				>
					{entry.path}
				</button>
			))}
		</div>
	)
}

export function ToolPreviewOverlay() {
	const { overlay, closeOverlay } = useToolOverlay()

	if (!overlay) return null

	if (overlay.type === "multi-diff") {
		return (
			<MultiDiffOverlay
				toolCalls={overlay.toolCalls}
				initialActiveId={overlay.activeId}
				onClose={closeOverlay}
			/>
		)
	}

	return <SingleToolOverlay toolCall={overlay.toolCall} onClose={closeOverlay} />
}

function SingleToolOverlay({
	toolCall,
	onClose,
}: {
	toolCall: import("@/types/acp").ToolCallState
	onClose: () => void
}) {
	const preview = buildToolPreview(toolCall)
	if (!preview) return null

	const subtitle = [preview.path, preview.lineRange].filter(Boolean).join(" · ")

	return (
		<OverlayShell
			open
			title={preview.title}
			subtitle={subtitle || undefined}
			badge={preview.badge}
			onClose={onClose}
		>
			{preview.kind === "diff" && preview.diff ? (
				<PierreFileDiff
					path={preview.diff.path}
					oldText={preview.diff.oldText}
					newText={preview.diff.newText}
					expanded
					className="rounded-none border-0"
				/>
			) : preview.kind === "terminal" ? (
				<div className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap text-foreground/90">
					<AnsiToHtml>{preview.code}</AnsiToHtml>
				</div>
			) : preview.kind === "code" ? (
				<div className="p-3">
					<CodeBlock language={preview.language} code={preview.code} variant="full" />
				</div>
			) : (
				<pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap text-foreground/90">
					{preview.code}
				</pre>
			)}
		</OverlayShell>
	)
}

function MultiDiffOverlay({
	toolCalls,
	initialActiveId,
	onClose,
}: {
	toolCalls: import("@/types/acp").ToolCallState[]
	initialActiveId?: string
	onClose: () => void
}) {
	const entries = useMemo(() => collectDiffTools(toolCalls), [toolCalls])
	const [activeId, setActiveId] = useState(initialActiveId ?? entries[0]?.id ?? "")
	const active = entries.find((entry) => entry.id === activeId) ?? entries[0]

	if (!active) return null

	return (
		<OverlayShell
			open
			title="Cambios del turno"
			subtitle={`${entries.length} archivos`}
			badge="Multi-diff"
			onClose={onClose}
		>
			<DiffTabs entries={entries} activeId={active.id} onSelect={setActiveId} />
			<PierreFileDiff
				path={active.path}
				oldText={active.oldText}
				newText={active.newText}
				expanded
				className="rounded-none border-0"
			/>
		</OverlayShell>
	)
}