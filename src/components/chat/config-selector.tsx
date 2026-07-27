import { useAtomValue } from "jotai"
import { setConfigOption } from "@/lib/tauri"
import { configOptionsAtom, promptInFlightAtom } from "@/stores/atoms"

export function ConfigSelectors() {
	const options = useAtomValue(configOptionsAtom)
	const promptInFlight = useAtomValue(promptInFlightAtom)

	if (options.length === 0) return null

	// Prefer select options that look like model/mode.
	const selects = options.filter((o) => o.options.length > 0).slice(0, 3)

	if (selects.length === 0) return null

	return (
		<div className="flex flex-wrap items-center gap-2">
			{selects.map((option) => (
				<label
					key={option.id}
					className="flex items-center gap-1.5 text-[11px] text-muted"
				>
					<span className="max-w-[80px] truncate">{option.name}</span>
					<select
						disabled={promptInFlight}
						value={option.currentValue}
						onChange={(e) => {
							void setConfigOption(option.id, e.target.value)
						}}
						className="max-w-[160px] rounded border border-border bg-surface px-1.5 py-0.5 text-[11px] text-fg outline-none disabled:opacity-50"
					>
						{option.options.map((item) => (
							<option key={item.value} value={item.value}>
								{item.name || item.value}
							</option>
						))}
					</select>
				</label>
			))}
		</div>
	)
}
