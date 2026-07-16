import { Plug, Sparkles, Terminal, type LucideIcon } from "lucide-react"
import { useMemo } from "react"
import { ComposerCommandMenu } from "@/components/chat/composer-command-menu"
import {
	CommandEmpty,
	CommandGroup,
	CommandItem,
	CommandList,
} from "@/components/ui/command"
import { filterSlashEntries, type SlashEntry } from "@/lib/slash-prompt"

interface SlashCommandPickerProps {
	query: string
	entries: SlashEntry[]
	selectedValue: string
	onSelect: (entry: SlashEntry) => void
	onValueChange: (value: string) => void
}

function slashEntryKey(entry: SlashEntry) {
	return `${entry.kind}:${entry.scope}:${entry.name}`
}

function slashEntryIcon(kind: SlashEntry["kind"]): LucideIcon {
	switch (kind) {
		case "command":
			return Terminal
		case "skill":
			return Sparkles
		case "mcp":
			return Plug
	}
}

function SlashEntryItem({
	entry,
	onSelect,
}: {
	entry: SlashEntry
	onSelect: (entry: SlashEntry) => void
}) {
	const Icon = slashEntryIcon(entry.kind)
	return (
		<CommandItem
			value={slashEntryKey(entry)}
			onSelect={() => onSelect(entry)}
		>
			<Icon className="size-4 shrink-0 text-muted-foreground" />
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
		</CommandItem>
	)
}

export function SlashCommandPicker({
	query,
	entries,
	selectedValue,
	onSelect,
	onValueChange,
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
	const mcps = useMemo(() => filtered.filter((entry) => entry.kind === "mcp"), [filtered])

	return (
		<ComposerCommandMenu value={selectedValue} onValueChange={onValueChange}>
			<CommandList>
				{commands.length > 0 ? (
					<CommandGroup heading="Commands">
						{commands.map((entry) => (
							<SlashEntryItem key={slashEntryKey(entry)} entry={entry} onSelect={onSelect} />
						))}
					</CommandGroup>
				) : null}
				{skills.length > 0 ? (
					<CommandGroup heading="Skills">
						{skills.map((entry) => (
							<SlashEntryItem key={slashEntryKey(entry)} entry={entry} onSelect={onSelect} />
						))}
					</CommandGroup>
				) : null}
				{mcps.length > 0 ? (
					<CommandGroup heading="MCP">
						{mcps.map((entry) => (
							<SlashEntryItem key={slashEntryKey(entry)} entry={entry} onSelect={onSelect} />
						))}
					</CommandGroup>
				) : null}
				{filtered.length === 0 ? (
					<CommandEmpty>No hay commands, skills ni MCPs que coincidan.</CommandEmpty>
				) : null}
			</CommandList>
		</ComposerCommandMenu>
	)
}

export { slashEntryKey }