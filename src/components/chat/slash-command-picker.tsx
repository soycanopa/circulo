import { Plug, Sparkles, Terminal, type LucideIcon } from "lucide-react"
import { useMemo } from "react"
import { ComposerCommandMenu } from "@/components/chat/composer-command-menu"
import {
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
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
	onQueryChange: (query: string) => void
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

export function SlashCommandPicker({
	query,
	entries,
	selectedValue,
	onSelect,
	onValueChange,
	onQueryChange,
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
		<ComposerCommandMenu>
			<Command
				className="w-full rounded-lg border"
				shouldFilter={false}
				loop
				value={selectedValue}
				onValueChange={onValueChange}
			>
				<CommandInput
					placeholder="Type a command or search..."
					value={query}
					onValueChange={onQueryChange}
				/>
				<CommandList>
					<CommandEmpty>No hay commands, skills ni MCPs que coincidan.</CommandEmpty>
					{commands.length > 0 ? (
						<CommandGroup heading="Commands">
							{commands.map((entry) => {
								const Icon = slashEntryIcon(entry.kind)
								return (
									<CommandItem
										key={slashEntryKey(entry)}
										value={slashEntryKey(entry)}
										onSelect={() => onSelect(entry)}
										title={entry.description ?? undefined}
									>
										<Icon />
										<span>/{entry.name}</span>
										<CommandShortcut>{entry.scope}</CommandShortcut>
									</CommandItem>
								)
							})}
						</CommandGroup>
					) : null}
					{showSeparatorBeforeSkills ? <CommandSeparator /> : null}
					{skills.length > 0 ? (
						<CommandGroup heading="Skills">
							{skills.map((entry) => {
								const Icon = slashEntryIcon(entry.kind)
								return (
									<CommandItem
										key={slashEntryKey(entry)}
										value={slashEntryKey(entry)}
										onSelect={() => onSelect(entry)}
										title={entry.description ?? undefined}
									>
										<Icon />
										<span>/{entry.name}</span>
										<CommandShortcut>{entry.scope}</CommandShortcut>
									</CommandItem>
								)
							})}
						</CommandGroup>
					) : null}
					{showSeparatorBeforeMcp ? <CommandSeparator /> : null}
					{mcps.length > 0 ? (
						<CommandGroup heading="MCP">
							{mcps.map((entry) => {
								const Icon = slashEntryIcon(entry.kind)
								return (
									<CommandItem
										key={slashEntryKey(entry)}
										value={slashEntryKey(entry)}
										onSelect={() => onSelect(entry)}
										title={entry.description ?? undefined}
									>
										<Icon />
										<span>/{entry.name}</span>
										<CommandShortcut>{entry.scope}</CommandShortcut>
									</CommandItem>
								)
							})}
						</CommandGroup>
					) : null}
				</CommandList>
			</Command>
		</ComposerCommandMenu>
	)
}

export { slashEntryKey }