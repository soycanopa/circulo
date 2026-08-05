export interface SlashCommand {
	/** Trigger token without the leading slash (e.g. `compact`). */
	command: string
	/** Text shown in the menu (usually `/<command>`). */
	label: string
	description: string
	/** How the command is executed. Defaults to sending the label as text. */
	action?: "text" | "new-chat" | "insert"
	/** Prompt sent to the agent; defaults to `label`. */
	prompt?: string
}

export const DEFAULT_SLASH_COMMANDS: SlashCommand[] = [
	{
		command: "compact",
		label: "/compact",
		description: "Compact the conversation context to save tokens",
	},
	{
		command: "help",
		label: "/help",
		description: "Show the agent's available commands",
	},
	{
		command: "clear",
		label: "/clear",
		description: "Start a fresh conversation",
		action: "new-chat",
	},
	{
		command: "mcp",
		label: "/mcp",
		description: "Load a registered MCP server on demand (circulo-mcp)",
	},
]

/**
 * Built-in commands plus user-defined ones. User commands can't override the
 * built-ins (`/compact`, `/help`, `/clear`). For custom commands the menu
 * shows the `/token` while `prompt` carries the full text sent to the agent.
 */
export function mergeSlashCommands(
	custom: { command: string; label: string; description: string }[] = [],
): SlashCommand[] {
	const customCommands = custom
		.filter(
			(item) =>
				!DEFAULT_SLASH_COMMANDS.some(
					(d) => d.command === item.command.replace(/^\//, ""),
				),
		)
		.map((item) => ({
			command: item.command.replace(/^\//, ""),
			label: item.command,
			description: item.description,
			prompt: item.label,
		}))
	return [...DEFAULT_SLASH_COMMANDS, ...customCommands]
}
