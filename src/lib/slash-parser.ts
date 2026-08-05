import type { SlashCommand } from "@/lib/slash-commands"

/**
 * Active `/query` token at the cursor. Slash commands only trigger when the
 * token starts at the very beginning of the composer input.
 */
export function getActiveSlash(
	text: string,
	cursor: number,
): { query: string; start: number } | null {
	const before = text.slice(0, cursor)
	const slash = before.lastIndexOf("/")
	if (slash !== 0) return null
	// No whitespace or newline inside the token, otherwise the command is done.
	if (/[\s\n]/.test(before)) return null
	const query = before.slice(1)
	return { query, start: slash }
}

export function filterSlashCommands(
	query: string,
	commands: SlashCommand[],
): SlashCommand[] {
	const q = query.toLowerCase()
	if (!q) return commands
	return commands.filter((command) =>
		command.command.toLowerCase().startsWith(q),
	)
}
