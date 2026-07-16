import { Plug, Sparkles, Terminal, type LucideIcon } from "lucide-react"
import { useMemo } from "react"
import { ComposerCommandMenu } from "@/components/chat/composer-command-menu"
import {
	CommandEmpty,
	CommandGroup,
	CommandItem,
	CommandList,
	CommandSeparator,
	CommandShortcut,
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
			title={entry.description ?? undefined}
		>
			<Icon />
			<span>/{entry.name}</span>
			<CommandShortcut>{entry.scope}</CommandShortcut>
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

	const showSeparatorBeforeSkills = commands.length > 0 && skills.length > 0
	const showSeparatorBeforeMcp =
		(commands.length > 0 || skills.length > 0) && mcps.length > 0

	return (
		<ComposerCommandMenu
			value={selectedValue}
			onValueChange={onValueChange}
			query={query ? `/${query}` : "/"}
			placeholder="Buscar commands, skills o MCP…"
		>
			<CommandList>
				<CommandEmpty>No hay commands, skills ni MCPs que coincidan.</CommandEmpty>
				{commands.length > 0 ? (
					<CommandGroup heading="Commands">
						{commands.map((entry) => (
							<SlashEntryItem key={slashEntryKey(entry)} entry={entry} onSelect={onSelect} />
						))}
					</CommandGroup>
				) : null}
				{showSeparatorBeforeSkills ? <CommandSeparator /> : null}
				{skills.length > 0 ? (
					<CommandGroup heading="Skills">
						{skills.map((entry) => (
							<SlashEntryItem key={slashEntryKey(entry)} entry={entry} onSelect={onSelect} />
						))}
					</CommandGroup>
				) : null}
				{showSeparatorBeforeMcp ? <CommandSeparator /> : null}
				{mcps.length > 0 ? (
					<CommandGroup heading="MCP">
						{mcps.map((entry) => (
							<SlashEntryItem key={slashEntryKey(entry)} entry={entry} onSelect={onSelect} />
						))}
					</CommandGroup>
				) : null}
			</CommandList>
		</ComposerCommandMenu>
	)
}

export { slashEntryKey }