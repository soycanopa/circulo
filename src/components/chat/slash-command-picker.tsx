import { Sparkles, Terminal } from "lucide-react"
import { useMemo } from "react"
import { filterSlashEntries, type SlashEntry } from "@/lib/slash-prompt"
import { cn } from "@/lib/utils"

interface SlashCommandPickerProps {
	query: string
	entries: SlashEntry[]
	selectedIndex: number
	onSelect: (entry: SlashEntry) => void
	onHover: (index: number) => void
}

export function SlashCommandPicker({
	query,
	entries,
	selectedIndex,
	onSelect,
	onHover,
}: SlashCommandPickerProps) {
	const filtered = useMemo(() => filterSlashEntries(entries, query), [entries, query])
	const commands = useMemo(
		() => filtered.filter((entry) => entry.kind === "command"),
		[filtered],
	)
	const skills = useMemo(
		() => filtered.filter((entry) => entry.kind === "skill"),
		[filtered],
	)

	if (filtered.length === 0) {
		return (
			<div className="absolute bottom-full left-0 z-20 mb-2 w-full overflow-hidden rounded-lg border border-popover-border bg-popover px-3 py-2 text-xs text-muted-foreground shadow-lg">
				No hay commands ni skills que coincidan.
			</div>
		)
	}

	return (
		<div className="absolute bottom-full left-0 z-20 mb-2 max-h-64 w-full overflow-y-auto rounded-lg border border-popover-border bg-popover shadow-lg">
			{commands.length > 0 ? (
				<SlashGroup
					label="Commands"
					entries={commands}
					selectedIndex={selectedIndex}
					startIndex={0}
					onSelect={onSelect}
					onHover={onHover}
				/>
			) : null}
			{skills.length > 0 ? (
				<SlashGroup
					label="Skills"
					entries={skills}
					selectedIndex={selectedIndex}
					startIndex={commands.length}
					onSelect={onSelect}
					onHover={onHover}
				/>
			) : null}
		</div>
	)
}

function SlashGroup({
	label,
	entries,
	selectedIndex,
	startIndex,
	onSelect,
	onHover,
}: {
	label: string
	entries: SlashEntry[]
	selectedIndex: number
	startIndex: number
	onSelect: (entry: SlashEntry) => void
	onHover: (index: number) => void
}) {
	return (
		<div>
			<p className="sticky top-0 border-b border-border/60 bg-popover px-3 py-1.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
				{label}
			</p>
			{entries.map((entry, index) => {
				const absoluteIndex = startIndex + index
				const Icon = entry.kind === "command" ? Terminal : Sparkles
				return (
					<button
						key={`${entry.kind}-${entry.scope}-${entry.name}`}
						type="button"
						className={cn(
							"flex w-full items-start gap-2 px-3 py-2 text-left text-sm transition-colors hover:bg-accent",
							selectedIndex === absoluteIndex && "bg-accent",
						)}
						onMouseEnter={() => onHover(absoluteIndex)}
						onClick={() => onSelect(entry)}
					>
						<Icon className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" />
						<span className="min-w-0 flex-1">
							<span className="font-medium text-foreground">/{entry.name}</span>
							{entry.description ? (
								<span className="mt-0.5 block truncate text-xs text-muted-foreground">
									{entry.description}
								</span>
							) : null}
							<span className="mt-0.5 block text-[10px] uppercase text-muted-foreground/80">
								{entry.scope}
							</span>
						</span>
					</button>
				)
			})}
		</div>
	)
}