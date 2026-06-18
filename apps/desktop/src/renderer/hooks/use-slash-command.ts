import { useCallback } from "react"
import { getProjectClient } from "../services/connection-manager"

export interface ExecuteSlashOptions {
	client: ReturnType<typeof getProjectClient>
	sessionID: string
	model?: { providerID: string; modelID: string }
	agent?: string
	onUndo?: () => Promise<string | undefined>
	onRedo?: () => Promise<void>
	onFork?: () => Promise<void>
	onSkillsOpen?: () => void
}

export interface SlashCommandResult {
	handled: boolean
	cmdName?: string
	cmdArgs?: string
}

export function useSlashCommand() {
		const executeSlashCommand = useCallback(
		async (
			text: string,
			options: ExecuteSlashOptions,
			/** Command-level agent override (e.g., from popover selection) */
			commandAgent?: string,
		): Promise<SlashCommandResult> => {
			const trimmed = text.trim()
			if (!trimmed.startsWith("/")) return { handled: false }

			const spaceIndex = trimmed.indexOf(" ")
			const cmdName = spaceIndex === -1 ? trimmed.slice(1) : trimmed.slice(1, spaceIndex)
			const cmdArgs = spaceIndex === -1 ? "" : trimmed.slice(spaceIndex + 1).trim()

			switch (cmdName.toLowerCase()) {
				case "undo":
					if (options.onUndo) await options.onUndo()
					return { handled: true, cmdName, cmdArgs }
				case "redo":
					if (options.onRedo) await options.onRedo()
					return { handled: true, cmdName, cmdArgs }
				case "compact":
				case "summarize":
					if (options.client && options.model) {
						try {
							await options.client.session.summarize({
								sessionID: options.sessionID,
								providerID: options.model.providerID,
								modelID: options.model.modelID,
							})
						} catch (_err) {}
					}
					return { handled: true, cmdName, cmdArgs }
				case "fork":
					if (options.onFork) {
						await options.onFork()
					}
					return { handled: true, cmdName, cmdArgs }
				case "skills":
					if (options.onSkillsOpen) {
						options.onSkillsOpen()
					}
					return { handled: true, cmdName, cmdArgs }
				default:
					break
			}

			if (options.client) {
				try {
					await options.client.session.command({
						sessionID: options.sessionID,
						command: cmdName,
						arguments: cmdArgs,
						agent: commandAgent ?? options.agent,
					})
					return { handled: true, cmdName, cmdArgs }
				} catch (_err) {}
			}

			return { handled: false, cmdName, cmdArgs }
		},
		[],
	)

	return { executeSlashCommand }
}
