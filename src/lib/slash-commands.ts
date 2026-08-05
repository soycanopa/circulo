export interface SlashCommand {
	command: string
	label: string
	description: string
	/** How the command is executed. Defaults to sending the label as text. */
	action?: "text" | "new-chat"
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
]
