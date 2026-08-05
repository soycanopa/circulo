import { useAtomValue, useSetAtom } from "jotai"
import { useState, type ReactNode } from "react"
import { Switch } from "@/components/ui/switch"
import { setAutoApprove } from "@/lib/tauri"
import { appSettingsAtom } from "@/stores/atoms"

export function SectionHeader({
	title,
	description,
}: {
	title: string
	description?: ReactNode
}) {
	return (
		<div className="mb-6">
			<h2 className="text-lg font-medium text-fg">{title}</h2>
			{description ? (
				<p className="mt-1 text-xs leading-snug text-muted">{description}</p>
			) : null}
		</div>
	)
}

export function SettingRow({
	label,
	description,
	control,
}: {
	label: string
	description?: ReactNode
	control?: ReactNode
}) {
	return (
		<div className="flex items-center justify-between gap-4 rounded-lg border border-border bg-black/20 px-3.5 py-3">
			<div className="min-w-0 flex-1">
				<div className="text-sm text-fg">{label}</div>
				{description ? (
					<div className="mt-0.5 text-[11px] leading-snug text-muted">
						{description}
					</div>
				) : null}
			</div>
			{control ? <div className="shrink-0">{control}</div> : null}
		</div>
	)
}

export function PathRow({ label, path }: { label: string; path: string | null }) {
	return (
		<SettingRow
			label={label}
			control={
				<code className="max-w-[16rem] truncate rounded bg-white/5 px-1.5 py-0.5 font-mono text-[10px] text-fg/90">
					{path ?? "…"}
				</code>
			}
		/>
	)
}

/** Shared Auto-edit toggle persisted via setAutoApprove. */
export function AutoEditSwitch({ className }: { className?: string }) {
	const appSettings = useAtomValue(appSettingsAtom)
	const setAppSettings = useSetAtom(appSettingsAtom)
	const [saving, setSaving] = useState(false)

	const enabled = appSettings?.autoApproveEnabled ?? false

	async function handleToggle(next: boolean) {
		setSaving(true)
		try {
			const settings = await setAutoApprove(next)
			setAppSettings(settings)
		} catch {
			// Best-effort: settings persistence failure keeps the current value.
		} finally {
			setSaving(false)
		}
	}

	return (
		<Switch
			checked={enabled}
			disabled={saving}
			onCheckedChange={(next) => void handleToggle(next)}
			aria-label="Auto-edit"
			className={className}
		/>
	)
}
