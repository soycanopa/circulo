import { useAtomValue, useSetAtom } from "jotai"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { setConfigOption } from "@/lib/tauri"
import { configOptionsAtom, promptInFlightAtom } from "@/stores/atoms"
import type { ConfigOption } from "@/types/acp"

/** Display order: mode → model → reasoning (if present). */
type ConfigKind = "mode" | "model" | "reasoning" | "other"

function classifyConfig(option: ConfigOption): ConfigKind {
	const haystack = [option.id, option.name, option.category ?? ""]
		.join(" ")
		.toLowerCase()

	// Reasoning / thinking effort (often depends on model).
	if (
		/\breason/.test(haystack) ||
		/\bthink/.test(haystack) ||
		/\beffort\b/.test(haystack) ||
		/\bintel/.test(haystack)
	) {
		return "reasoning"
	}
	// Session / agent mode (build, plan, etc.) — before generic "model".
	if (
		/\bmode\b/.test(haystack) ||
		/\bagent\b/.test(haystack) ||
		haystack.includes("permission")
	) {
		return "mode"
	}
	if (/\bmodel\b/.test(haystack) || /\bllm\b/.test(haystack)) {
		return "model"
	}
	return "other"
}

const KIND_ORDER: Record<ConfigKind, number> = {
	mode: 0,
	model: 1,
	reasoning: 2,
	other: 3,
}

function normalizeOption(option: ConfigOption): ConfigOption | null {
	// Boolean config → Yes/No select so it still shows in the toolbar.
	if (option.options.length === 0) {
		const current = option.currentValue.toLowerCase()
		if (current === "true" || current === "false" || current === "") {
			return {
				...option,
				currentValue:
					current === "true" || current === "false" ? current : "false",
				options: [
					{ value: "true", name: "On" },
					{ value: "false", name: "Off" },
				],
			}
		}
		return null
	}
	// Radix forbids empty item values.
	const cleaned = option.options.filter((item) => item.value.length > 0)
	if (cleaned.length === 0) return null
	return { ...option, options: cleaned }
}

/**
 * Mode, then model, then reasoning (only if the agent exposes it).
 * Other config options are omitted from this compact toolbar.
 */
function orderedToolbarOptions(options: ConfigOption[]): ConfigOption[] {
	const normalized = options
		.map(normalizeOption)
		.filter((option): option is ConfigOption => option !== null)

	const byKind = new Map<ConfigKind, ConfigOption>()
	for (const option of normalized) {
		const kind = classifyConfig(option)
		if (kind === "other") continue
		// Keep first match per kind (agents rarely send duplicates).
		if (!byKind.has(kind)) byKind.set(kind, option)
	}

	return (["mode", "model", "reasoning"] as const)
		.map((kind) => byKind.get(kind))
		.filter((option): option is ConfigOption => option !== undefined)
		// Stable secondary sort if we ever show multiple later.
		.sort(
			(a, b) =>
				KIND_ORDER[classifyConfig(a)] - KIND_ORDER[classifyConfig(b)],
		)
}

export function ConfigSelectors() {
	const options = useAtomValue(configOptionsAtom)
	const promptInFlight = useAtomValue(promptInFlightAtom)
	const setConfig = useSetAtom(configOptionsAtom)

	const selects = orderedToolbarOptions(options)

	if (selects.length === 0) return null

	return (
		<div className="flex min-w-0 flex-wrap items-center gap-1 pl-1.5">
			{selects.map((option) => {
				const values = new Set(option.options.map((item) => item.value))
				const value = values.has(option.currentValue)
					? option.currentValue
					: undefined

				return (
					<div key={option.id} className="shrink-0">
						<Select
							disabled={promptInFlight}
							value={value}
							onValueChange={(next) => {
								// Optimistic UI — agent acp:config_options will confirm.
								setConfig((current) =>
									current.map((entry) =>
										entry.id === option.id
											? { ...entry, currentValue: next }
											: entry,
									),
								)
								void setConfigOption(option.id, next)
							}}
						>
							<SelectTrigger
								aria-label={option.name}
								className="h-auto w-auto min-w-0 max-w-[14rem] gap-0.5 rounded-md border-0 bg-transparent px-1.5 py-1 text-[13px] text-white/70 shadow-none transition-colors hover:bg-white/[0.08] hover:text-white/90 focus-visible:border-0 focus-visible:bg-white/[0.08] focus-visible:ring-0 data-[state=open]:bg-white/[0.08] data-[placeholder]:text-white/50 [&>svg]:size-3.5 [&>svg]:text-white/55"
							>
								<SelectValue placeholder={option.name} />
							</SelectTrigger>
							<SelectContent position="popper" align="start">
								{option.options.map((item) => (
									<SelectItem key={item.value} value={item.value}>
										{item.name || item.value}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</div>
				)
			})}
		</div>
	)
}
