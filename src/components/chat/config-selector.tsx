import { useAtomValue, useSetAtom } from "jotai"
import { useCallback, useRef, useState } from "react"
import { ContextIndicator } from "@/components/chat/context-indicator"
import { AutoApproveToggle } from "@/components/chat/auto-approve-toggle"
import { ModelSelector } from "@/components/chat/model-selector"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { setConfigOption, setFavoriteModel } from "@/lib/tauri"
import {
	activeSessionIdAtom,
	appSettingsAtom,
	sessionsAtom,
	visibleConfigOptionsAtom,
	visibleContextUsageAtom,
	visiblePromptInFlightAtom,
} from "@/stores/atoms"
import type { ConfigOption } from "@/types/acp"

/** Display order: mode → model → reasoning (if present). */
type ConfigKind = "mode" | "model" | "reasoning" | "other"

function classifyConfig(option: ConfigOption): ConfigKind {
	const category = (option.category ?? "").toLowerCase()
	if (category === "thought_level" || category === "reasoning") {
		return "reasoning"
	}
	if (category === "mode") return "mode"
	if (category === "model" || category === "model_config") return "model"

	const haystack = [option.id, option.name, option.category ?? ""]
		.join(" ")
		.toLowerCase()

	if (
		/\breason/.test(haystack) ||
		/\bthink/.test(haystack) ||
		/\beffort\b/.test(haystack) ||
		/\bintel/.test(haystack)
	) {
		return "reasoning"
	}
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
	const cleaned = option.options.filter((item) => item.value.length > 0)
	if (cleaned.length === 0) return null
	return { ...option, options: cleaned }
}

function orderedToolbarOptions(options: ConfigOption[]): ConfigOption[] {
	const normalized = options
		.map(normalizeOption)
		.filter((option): option is ConfigOption => option !== null)

	const byKind = new Map<ConfigKind, ConfigOption>()
	for (const option of normalized) {
		const kind = classifyConfig(option)
		if (kind === "other") continue
		if (!byKind.has(kind)) byKind.set(kind, option)
	}

	return (["mode", "model", "reasoning"] as const)
		.map((kind) => byKind.get(kind))
		.filter((option): option is ConfigOption => option !== undefined)
		.sort(
			(a, b) =>
				KIND_ORDER[classifyConfig(a)] - KIND_ORDER[classifyConfig(b)],
		)
}

function isModelSelected(option: ConfigOption): boolean {
	const value = option.currentValue.trim()
	if (!value) return false
	return option.options.some((item) => item.value === value)
}

function shouldShowContextAfter(option: ConfigOption, selects: ConfigOption[]): boolean {
	const kind = classifyConfig(option)
	if (kind === "reasoning") return true
	if (kind === "model") {
		return !selects.some((entry) => classifyConfig(entry) === "reasoning")
	}
	return false
}

function CompactConfigSelect({
	option,
	promptInFlight,
	activeSessionId,
	setSessions,
}: {
	option: ConfigOption
	promptInFlight: boolean
	activeSessionId: string | null
	setSessions: ReturnType<typeof useSetAtom<typeof sessionsAtom>>
}) {
	const values = new Set(option.options.map((item) => item.value))
	const value = values.has(option.currentValue) ? option.currentValue : undefined

	return (
		<div className="shrink-0">
			<Select
				disabled={promptInFlight}
				value={value}
				onValueChange={(next) => {
					const targetSid = activeSessionId
					if (targetSid) {
						setSessions((prev) => {
							const current = prev[targetSid]
							if (!current) return prev
							return {
								...prev,
								[targetSid]: {
									...current,
									configOptions: current.configOptions.map((entry) =>
										entry.id === option.id
											? { ...entry, currentValue: next }
											: entry,
									),
								},
							}
						})
					}
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
}

export function ConfigSelectors() {
	const options = useAtomValue(visibleConfigOptionsAtom)
	const contextUsage = useAtomValue(visibleContextUsageAtom)
	const promptInFlight = useAtomValue(visiblePromptInFlightAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const activeSessionId = useAtomValue(activeSessionIdAtom)
	const appSettings = useAtomValue(appSettingsAtom)
	const setAppSettings = useSetAtom(appSettingsAtom)
	const [optimisticFavorites, setOptimisticFavorites] = useState<
		string[] | null
	>(null)
	const [pendingFavoriteIds, setPendingFavoriteIds] = useState<Set<string>>(
		() => new Set(),
	)
	const favoriteRequestVersion = useRef(0)

	const favoriteModelIds =
		optimisticFavorites ?? appSettings?.favoriteModelIds ?? []

	const handleToggleFavorite = useCallback(
		async (modelId: string, favorite: boolean) => {
			if (pendingFavoriteIds.has(modelId)) return

			const requestVersion = ++favoriteRequestVersion.current
			const base = optimisticFavorites ?? appSettings?.favoriteModelIds ?? []
			const next = favorite
				? [...base.filter((id) => id !== modelId), modelId]
				: base.filter((id) => id !== modelId)

			setPendingFavoriteIds((prev) => new Set(prev).add(modelId))
			setOptimisticFavorites(next)

			try {
				const settings = await setFavoriteModel(modelId, favorite)
				if (requestVersion === favoriteRequestVersion.current) {
					setAppSettings(settings)
					setOptimisticFavorites(null)
				}
			} catch {
				if (requestVersion === favoriteRequestVersion.current) {
					setOptimisticFavorites(null)
				}
			} finally {
				setPendingFavoriteIds((prev) => {
					const copy = new Set(prev)
					copy.delete(modelId)
					return copy
				})
			}
		},
		[
			appSettings?.favoriteModelIds,
			optimisticFavorites,
			pendingFavoriteIds,
			setAppSettings,
		],
	)

	const selects = orderedToolbarOptions(options)
	const modelOption = selects.find((option) => classifyConfig(option) === "model")
	const showContextIndicator =
		modelOption !== undefined && isModelSelected(modelOption)

	if (selects.length === 0) return null

	return (
		<div className="flex min-w-0 flex-wrap items-center gap-1 pl-1.5">
			{selects.flatMap((option) => {
				const kind = classifyConfig(option)
				const items = [
					kind === "model" ? (
						<ModelSelector
							key={option.id}
							option={option}
							favoriteModelIds={favoriteModelIds}
							pendingFavoriteIds={pendingFavoriteIds}
							onToggleFavorite={handleToggleFavorite}
						/>
					) : (
						<CompactConfigSelect
							key={option.id}
							option={option}
							promptInFlight={promptInFlight}
							activeSessionId={activeSessionId}
							setSessions={setSessions}
						/>
					),
				]

				if (
					showContextIndicator &&
					shouldShowContextAfter(option, selects)
				) {
					items.push(
						<ContextIndicator
							key="context-usage"
							usage={contextUsage}
							className="mx-0.5"
						/>,
						<AutoApproveToggle key="auto-approve" className="mx-0.5" />,
					)
				}

				return items
			})}
		</div>
	)
}
