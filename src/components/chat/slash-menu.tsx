import { Command } from "lucide-react"
import { cn } from "@/lib/utils"
import type { SlashCommand } from "@/lib/slash-commands"

interface SlashMenuProps {
	results: SlashCommand[]
	selectedIndex: number
	onSelect: (command: SlashCommand) => void
}

export function SlashMenu({
	results,
	selectedIndex,
	onSelect,
}: SlashMenuProps) {
	return (
		<div className="frosted-strong absolute bottom-full left-0 right-0 z-20 mb-1 overflow-hidden rounded-md border border-border shadow-lg">
			<div className="border-b border-border px-2.5 py-1.5 text-[10px] uppercase tracking-wider text-muted">
				Commands
			</div>
			{results.length === 0 ? (
				<p className="px-2.5 py-2 text-xs text-muted">No matching commands</p>
			) : (
				<ul className="max-h-48 overflow-y-auto py-1">
					{results.map((item, index) => (
						<li key={item.command}>
							<button
								type="button"
								onMouseDown={(event) => {
									// Keep focus on the textarea.
									event.preventDefault()
									onSelect(item)
								}}
								className={cn(
									"flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs text-fg transition",
									index === selectedIndex
										? "bg-white/10"
										: "hover:bg-white/5",
								)}
							>
								<Command className="size-3.5 shrink-0 text-muted" />
								<span className="font-medium">{item.label}</span>
								<span className="min-w-0 flex-1 truncate text-muted">
									{item.description}
								</span>
							</button>
						</li>
					))}
				</ul>
			)}
		</div>
	)
}
