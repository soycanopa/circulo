import { Trash2 } from "lucide-react"
import { useState } from "react"
import { SectionHeader } from "@/components/settings/sections/section-ui"
import type { CustomSlashCommand } from "@/types/acp"

interface SlashCommandsSectionProps {
	customSlashCommands?: CustomSlashCommand[]
	onSaveSlashCommand?: (
		command: string,
		label: string,
		description: string,
	) => Promise<void>
	onDeleteSlashCommand?: (command: string) => Promise<void>
}

export function SlashCommandsSection({
	customSlashCommands = [],
	onSaveSlashCommand,
	onDeleteSlashCommand,
}: SlashCommandsSectionProps) {
	const [command, setCommand] = useState("")
	const [label, setLabel] = useState("")
	const [description, setDescription] = useState("")
	const [error, setError] = useState<string | null>(null)
	const [saving, setSaving] = useState(false)

	async function handleSave() {
		setError(null)
		const trimmed = command.trim()
		const labelValue = label.trim()
		if (!trimmed || !labelValue) return
		if (!trimmed.startsWith("/")) {
			setError("Command must start with '/'")
			return
		}
		setSaving(true)
		try {
			await onSaveSlashCommand?.(trimmed, labelValue, description.trim())
			setCommand("")
			setLabel("")
			setDescription("")
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to save command")
		} finally {
			setSaving(false)
		}
	}

	async function handleDelete(itemCommand: string) {
		setSaving(true)
		try {
			await onDeleteSlashCommand?.(itemCommand)
		} finally {
			setSaving(false)
		}
	}

	return (
		<div>
			<SectionHeader
				title="Slash commands"
				description="Type the token in the composer to run its prompt."
			/>
			<div className="space-y-3">
				{customSlashCommands.length === 0 ? (
					<p className="text-xs text-muted">No custom slash commands yet.</p>
				) : (
					<div className="space-y-1.5">
						{customSlashCommands.map((item) => (
							<div
								key={item.command}
								className="flex items-center justify-between gap-2 rounded-lg border border-border bg-black/20 px-3.5 py-2.5"
							>
								<div className="min-w-0 flex-1">
									<span className="font-mono text-sm text-fg">
										{item.command}
									</span>
									{item.description ? (
										<p className="mt-0.5 truncate text-[11px] text-muted">
											{item.description}
										</p>
									) : null}
								</div>
								<button
									type="button"
									disabled={saving}
									onClick={() => void handleDelete(item.command)}
									className="shrink-0 rounded p-1.5 text-muted hover:bg-white/5 hover:text-red-300 disabled:opacity-40"
									title="Delete slash command"
								>
									<Trash2 className="size-3.5" />
								</button>
							</div>
						))}
					</div>
				)}

				<div className="rounded-lg border border-border bg-black/20 p-3.5">
					<div className="mb-2 text-[11px] uppercase tracking-wider text-muted">
						New slash command
					</div>
					<div className="space-y-2">
						<input
							value={command}
							onChange={(event) => setCommand(event.target.value)}
							placeholder="/review"
							className="w-full rounded-md border border-border bg-black/20 px-2 py-1.5 font-mono text-sm text-fg"
						/>
						<input
							value={label}
							onChange={(event) => setLabel(event.target.value)}
							placeholder="Prompt to send"
							className="w-full rounded-md border border-border bg-black/20 px-2 py-1.5 text-sm text-fg"
						/>
						<input
							value={description}
							onChange={(event) => setDescription(event.target.value)}
							placeholder="Description (optional)"
							className="w-full rounded-md border border-border bg-black/20 px-2 py-1.5 text-sm text-fg"
						/>
						{error ? <p className="text-[11px] text-red-300">{error}</p> : null}
						<button
							type="button"
							disabled={
								saving || !command.trim() || !label.trim() || !onSaveSlashCommand
							}
							onClick={() => void handleSave()}
							className="rounded-md border border-border px-2 py-1 text-[11px] text-fg transition hover:bg-white/5 disabled:opacity-40"
						>
							Save slash command
						</button>
					</div>
				</div>
			</div>
		</div>
	)
}
