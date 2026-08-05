import { Trash2 } from "lucide-react"
import { useState } from "react"
import {
	AutoEditSwitch,
	SectionHeader,
	SettingRow,
} from "@/components/settings/sections/section-ui"

interface PermissionsSectionProps {
	allowedToolPatterns?: string[]
	onSetAllowedTool?: (pattern: string, enabled: boolean) => Promise<void>
}

export function PermissionsSection({
	allowedToolPatterns = [],
	onSetAllowedTool,
}: PermissionsSectionProps) {
	const [newPattern, setNewPattern] = useState("")
	const [saving, setSaving] = useState(false)

	async function handleAdd() {
		const pattern = newPattern.trim()
		if (!pattern) return
		setSaving(true)
		try {
			await onSetAllowedTool?.(pattern, true)
			setNewPattern("")
		} finally {
			setSaving(false)
		}
	}

	async function handleRemove(pattern: string) {
		setSaving(true)
		try {
			await onSetAllowedTool?.(pattern, false)
		} finally {
			setSaving(false)
		}
	}

	return (
		<div>
			<SectionHeader
				title="Permissions"
				description="Control when the agent needs to ask before running tools."
			/>
			<div className="space-y-3">
				<SettingRow
					label="Auto-edit"
					description="Skip the permission prompt for edits made by the agent."
					control={<AutoEditSwitch />}
				/>

				<div className="rounded-lg border border-border bg-black/20 p-3.5">
					<div className="text-[11px] uppercase tracking-wider text-muted">
						Always allow tools
					</div>
					<p className="mt-1 text-[11px] leading-snug text-muted">
						Tools matching these patterns (exact or{" "}
						<code className="rounded bg-white/5 px-1">*</code> glob) skip the
						permission prompt. Use the bookmark button on a permission card to
						add one.
					</p>
					{allowedToolPatterns.length > 0 ? (
						<ul className="mt-2 space-y-1">
							{allowedToolPatterns.map((pattern) => (
								<li
									key={pattern}
									className="flex items-center justify-between gap-2 rounded-md border border-border bg-black/20 px-2 py-1.5"
								>
									<span className="truncate font-mono text-xs text-fg/90">
										{pattern}
									</span>
									<button
										type="button"
										disabled={saving}
										onClick={() => void handleRemove(pattern)}
										className="shrink-0 rounded p-1 text-muted hover:bg-white/5 hover:text-red-300 disabled:opacity-40"
										title="Forget pattern"
									>
										<Trash2 className="size-3.5" />
									</button>
								</li>
							))}
						</ul>
					) : (
						<p className="mt-2 text-xs text-muted">No remembered tools yet.</p>
					)}
					<div className="mt-2 flex gap-1.5">
						<input
							value={newPattern}
							onChange={(event) => setNewPattern(event.target.value)}
							onKeyDown={(event) => {
								if (event.key === "Enter") void handleAdd()
							}}
							placeholder="e.g. bash or edit*"
							className="w-full rounded-md border border-border bg-black/20 px-2 py-1.5 text-sm text-fg"
						/>
						<button
							type="button"
							disabled={saving || !newPattern.trim() || !onSetAllowedTool}
							onClick={() => void handleAdd()}
							className="shrink-0 rounded-md border border-border px-2 py-1 text-[11px] text-fg transition hover:bg-white/5 disabled:opacity-40"
						>
							Add
						</button>
					</div>
				</div>
			</div>
		</div>
	)
}
