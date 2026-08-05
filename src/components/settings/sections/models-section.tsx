import { useAtomValue, useSetAtom } from "jotai"
import { Clock3, Search, Star } from "lucide-react"
import { useMemo, useState } from "react"
import { ProviderIcon } from "@/components/chat/provider-icon"
import { SectionHeader } from "@/components/settings/sections/section-ui"
import { Input } from "@/components/ui/input"
import {
	extractRecentModels,
	groupModelOptions,
	modelMatchesQuery,
	modelShortName,
} from "@/lib/model-groups"
import { providerIdFromGroupOrValue } from "@/lib/provider-registry"
import { setFavoriteModel } from "@/lib/tauri"
import { appSettingsAtom, visibleConfigOptionsAtom } from "@/stores/atoms"
import type { ConfigOption, ConfigOptionValue } from "@/types/acp"
import { cn } from "@/lib/utils"

interface ModelsSectionProps {
	favoriteModelIds?: string[]
	recentModelIds?: string[]
}

function findModelOption(options: ConfigOption[]): ConfigOption | null {
	for (const option of options) {
		const category = (option.category ?? "").toLowerCase()
		if (category === "model" || category === "model_config") return option
		const haystack = [option.id, option.name, option.category ?? ""]
			.join(" ")
			.toLowerCase()
		if (/\bmodel\b/.test(haystack) || /\bllm\b/.test(haystack)) return option
	}
	return null
}

function ModelRow({
	item,
	favorite,
	onToggleFavorite,
	showProvider,
	pending,
}: {
	item: ConfigOptionValue
	favorite: boolean
	onToggleFavorite: (value: string, next: boolean) => void
	showProvider?: boolean
	pending?: boolean
}) {
	return (
		<div className="group flex items-center gap-2 rounded-md px-2 py-1.5 transition-colors hover:bg-white/[0.06]">
			<ProviderIcon
				providerId={providerIdFromGroupOrValue(item.group, item.value)}
			/>
			<span className="min-w-0 flex-1 truncate text-xs text-fg/90">
				{modelShortName(item)}
			</span>
			{showProvider && item.group ? (
				<span className="truncate text-[10px] text-muted">{item.group}</span>
			) : null}
			<button
				type="button"
				disabled={pending}
				title={favorite ? "Remove from favorites" : "Add to favorites"}
				onClick={() => onToggleFavorite(item.value, !favorite)}
				className="rounded p-1 text-white/35 transition-colors hover:bg-white/5 hover:text-white/70 disabled:opacity-40"
			>
				<Star
					className={cn(
						"size-3.5",
						favorite ? "fill-white/90 text-white/90" : "fill-none",
					)}
				/>
			</button>
		</div>
	)
}

function BareModelRow({
	value,
	favorite,
	onToggleFavorite,
}: {
	value: string
	favorite: boolean
	onToggleFavorite: (value: string, next: boolean) => void
}) {
	return (
		<div className="group flex items-center gap-2 rounded-md px-2 py-1.5 transition-colors hover:bg-white/[0.06]">
			<code className="min-w-0 flex-1 truncate font-mono text-[11px] text-fg/90">
				{value}
			</code>
			<button
				type="button"
				title={favorite ? "Remove from favorites" : "Add to favorites"}
				onClick={() => onToggleFavorite(value, !favorite)}
				className="rounded p-1 text-white/35 transition-colors hover:bg-white/5 hover:text-white/70"
			>
				<Star
					className={cn(
						"size-3.5",
						favorite ? "fill-white/90 text-white/90" : "fill-none",
					)}
				/>
			</button>
		</div>
	)
}

export function ModelsSection({
	favoriteModelIds = [],
	recentModelIds = [],
}: ModelsSectionProps) {
	const configOptions = useAtomValue(visibleConfigOptionsAtom)
	const setAppSettings = useSetAtom(appSettingsAtom)
	const [searchQuery, setSearchQuery] = useState("")
	const [pending, setPending] = useState<ReadonlySet<string>>(new Set())

	const modelOption = useMemo(
		() => findModelOption(configOptions),
		[configOptions],
	)
	const allModels = modelOption?.options ?? []

	const { favorites, groups } = useMemo(
		() => groupModelOptions(allModels, favoriteModelIds),
		[allModels, favoriteModelIds],
	)
	const recents = useMemo(
		() =>
			extractRecentModels(allModels, recentModelIds).filter(
				(item) => !favoriteModelIds.includes(item.value),
			),
		[allModels, recentModelIds, favoriteModelIds],
	)

	const favoriteSet = useMemo(
		() => new Set(favoriteModelIds),
		[favoriteModelIds],
	)

	// Favorites/recent ids not present in the live config (no session open).
	const offlineFavorites = favoriteModelIds.filter(
		(id) => !allModels.some((item) => item.value === id),
	)
	const offlineRecents = recentModelIds.filter(
		(id) =>
			!allModels.some((item) => item.value === id) &&
			!favoriteModelIds.includes(id),
	)

	const filteredGroups = useMemo(
		() =>
			groups
				.map((group) => ({
					...group,
					models: group.models.filter((item) =>
						modelMatchesQuery(item, searchQuery),
					),
				}))
				.filter((group) => group.models.length > 0),
		[groups, searchQuery],
	)

	async function handleToggleFavorite(value: string, next: boolean) {
		setPending((prev) => new Set(prev).add(value))
		try {
			const settings = await setFavoriteModel(value, next)
			setAppSettings(settings)
		} finally {
			setPending((prev) => {
				const rest = new Set(prev)
				rest.delete(value)
				return rest
			})
		}
	}

	const hasLiveModels = allModels.length > 0

	return (
		<div>
			<SectionHeader
				title="Models"
				description="Star models to pin them to the top of the composer selector."
			/>
			<div className="space-y-6">
				{!hasLiveModels && offlineFavorites.length === 0 ? (
					<p className="text-xs text-muted">
						Open a chat to load the available models for the active agent.
					</p>
				) : null}

				{favorites.length > 0 || offlineFavorites.length > 0 ? (
					<div>
						<div className="mb-1.5 text-[11px] uppercase tracking-wider text-muted">
							Favorites
						</div>
						<div className="space-y-0.5">
							{favorites.map((item) => (
								<ModelRow
									key={item.value}
									item={item}
									favorite
									onToggleFavorite={(value, next) =>
										void handleToggleFavorite(value, next)
									}
								/>
							))}
							{offlineFavorites.map((value) => (
								<BareModelRow
									key={value}
									value={value}
									favorite
									onToggleFavorite={(v, next) =>
										void handleToggleFavorite(v, next)
									}
								/>
							))}
						</div>
					</div>
				) : null}

				{recents.length > 0 || offlineRecents.length > 0 ? (
					<div>
						<div className="mb-1.5 flex items-center gap-1.5 text-[11px] uppercase tracking-wider text-muted">
							<Clock3 className="size-3" />
							Recents
						</div>
						<div className="space-y-0.5">
							{recents.map((item) => (
								<ModelRow
									key={item.value}
									item={item}
									favorite={false}
									onToggleFavorite={(value, next) =>
										void handleToggleFavorite(value, next)
									}
								/>
							))}
							{offlineRecents.map((value) => (
								<BareModelRow
									key={value}
									value={value}
									favorite={false}
									onToggleFavorite={(v, next) =>
										void handleToggleFavorite(v, next)
									}
								/>
							))}
						</div>
					</div>
				) : null}

				{hasLiveModels ? (
					<div>
						<div className="mb-2 text-[11px] uppercase tracking-wider text-muted">
							All models
						</div>
						<div className="relative mb-2">
							<Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted" />
							<Input
								type="search"
								value={searchQuery}
								placeholder="Search models…"
								autoComplete="off"
								className="h-8 pl-8"
								onChange={(event) => setSearchQuery(event.target.value)}
							/>
						</div>
						{filteredGroups.length === 0 ? (
							<p className="px-2 py-4 text-center text-xs text-muted">
								No models match “{searchQuery}”.
							</p>
						) : (
							<div className="space-y-3">
								{filteredGroups.map((group) => (
									<div key={`${group.providerId}-${group.providerLabel}`}>
										<div className="mb-1 px-2 text-[10px] font-semibold uppercase tracking-wider text-white/45">
											{group.providerLabel}
										</div>
										<div className="space-y-0.5">
											{group.models.map((item) => (
												<ModelRow
													key={item.value}
													item={item}
													favorite={favoriteSet.has(item.value)}
													pending={pending.has(item.value)}
													onToggleFavorite={(value, next) =>
														void handleToggleFavorite(value, next)
													}
												/>
											))}
										</div>
									</div>
								))}
							</div>
						)}
					</div>
				) : null}
			</div>
		</div>
	)
}
