import { Trash2 } from "lucide-react"
import { useState } from "react"
import { SectionHeader } from "@/components/settings/sections/section-ui"
import { saveAutomation } from "@/lib/tauri"
import type { Automation } from "@/types/acp"

interface AutomationsSectionProps {
	automations: Automation[]
	onAutomationsChange: () => void
	onDeleteAutomation: (id: string) => Promise<void>
}

export function AutomationsSection({
	automations,
	onAutomationsChange,
	onDeleteAutomation,
}: AutomationsSectionProps) {
	const [title, setTitle] = useState("")
	const [prompt, setPrompt] = useState("")
	const [saving, setSaving] = useState(false)

	async function handleSave() {
		if (!title.trim() || !prompt.trim()) return
		setSaving(true)
		try {
			await saveAutomation(title.trim(), prompt.trim())
			setTitle("")
			setPrompt("")
			onAutomationsChange()
		} finally {
			setSaving(false)
		}
	}

	return (
		<div>
			<SectionHeader
				title="Automations"
				description="Saved prompts that appear in the command palette (⌘K) for one-click runs."
			/>
			<div className="space-y-3">
				{automations.length === 0 ? (
					<p className="text-xs text-muted">No automations yet.</p>
				) : (
					<div className="space-y-1.5">
						{automations.map((item) => (
							<div
								key={item.id}
								className="flex items-center justify-between gap-2 rounded-lg border border-border bg-black/20 px-3.5 py-2.5"
							>
								<div className="min-w-0 flex-1">
									<div className="truncate text-sm text-fg">{item.title}</div>
									<div className="mt-0.5 line-clamp-2 text-[11px] text-muted">
										{item.prompt}
									</div>
								</div>
								<button
									type="button"
									onClick={() => void onDeleteAutomation(item.id)}
									className="shrink-0 rounded p-1.5 text-muted hover:bg-white/5 hover:text-red-300"
									title="Delete automation"
								>
									<Trash2 className="size-3.5" />
								</button>
							</div>
						))}
					</div>
				)}

				<div className="rounded-lg border border-border bg-black/20 p-3.5">
					<div className="mb-2 text-[11px] uppercase tracking-wider text-muted">
						New automation
					</div>
					<div className="space-y-2">
						<input
							value={title}
							onChange={(event) => setTitle(event.target.value)}
							placeholder="Title"
							className="w-full rounded-md border border-border bg-black/20 px-2 py-1.5 text-sm text-fg"
						/>
						<textarea
							value={prompt}
							onChange={(event) => setPrompt(event.target.value)}
							placeholder="Prompt to send"
							rows={3}
							className="w-full resize-none rounded-md border border-border bg-black/20 px-2 py-1.5 text-sm text-fg"
						/>
						<button
							type="button"
							disabled={saving || !title.trim() || !prompt.trim()}
							onClick={() => void handleSave()}
							className="rounded-md border border-border px-2 py-1 text-[11px] text-fg transition hover:bg-white/5 disabled:opacity-40"
						>
							Save automation
						</button>
					</div>
				</div>
			</div>
		</div>
	)
}
