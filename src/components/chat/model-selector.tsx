import { useAtomValue, useSetAtom } from "jotai"
import { Check, ChevronDown, Clock3, Search, Star } from "lucide-react"
import { useMemo, useState } from "react"
import { ProviderIcon } from "@/components/chat/provider-icon"
import { Input } from "@/components/ui/input"
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "@/components/ui/popover"
import {
	currentModelLabel,
	extractRecentModels,
	groupModelOptions,
	modelMatchesQuery,
	modelShortName,
} from "@/lib/model-groups"
import { providerIdFromGroupOrValue } from "@/lib/provider-registry"
import { cn } from "@/lib/utils"
import { markModelUsed, setConfigOption } from "@/lib/tauri"
import {
	activeSessionIdAtom,
	sessionsAtom,
	visiblePromptInFlightAtom,
} from "@/stores/atoms"
import type { ConfigOption, ConfigOptionValue } from "@/types/acp"

interface ModelSelectorProps {
	option: ConfigOption
	favoriteModelIds: string[]
	recentModelIds?: string[]
	pendingFavoriteIds?: ReadonlySet<string>
	onToggleFavorite: (modelId: string, favorite: boolean) => void | Promise<void>
}

const GROUP_LABEL_CLASS =
	"px-2 pb-1.5 pt-4 text-[10px] font-semibold uppercase tracking-wider text-white/45 first:pt-2"

export function ModelSelector({
	option,
	favoriteModelIds,
	recentModelIds = [],
	pendingFavoriteIds,
	onToggleFavorite,
}: ModelSelectorProps) {
	const promptInFlight = useAtomValue(visiblePromptInFlightAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const activeSessionId = useAtomValue(activeSessionIdAtom)
	const [open, setOpen] = useState(false)
	const [searchQuery, setSearchQuery] = useState("")

	const values = new Set(option.options.map((item) => item.value))
	const value = values.has(option.currentValue) ? option.currentValue : undefined

	const { favorites, groups } = useMemo(
		() => groupModelOptions(option.options, favoriteModelIds),
		[option.options, favoriteModelIds],
	)

	const favoriteSet = useMemo(
		() => new Set(favoriteModelIds),
		[favoriteModelIds],
	)

	const recentSet = useMemo(
		() => new Set(recentModelIds),
		[recentModelIds],
	)

	const recents = useMemo(
		() =>
			extractRecentModels(option.options, recentModelIds).filter(
				(item) => !favoriteSet.has(item.value),
			),
		[option.options, recentModelIds, favoriteSet],
	)

	const filteredFavorites = useMemo(
		() => favorites.filter((item) => modelMatchesQuery(item, searchQuery)),
		[favorites, searchQuery],
	)

	const filteredRecents = useMemo(
		() => recents.filter((item) => modelMatchesQuery(item, searchQuery)),
		[recents, searchQuery],
	)

	const filteredGroups = useMemo(
		() =>
			groups
				.map((group) => ({
					...group,
					models: group.models.filter(
						(item) =>
							modelMatchesQuery(item, searchQuery) &&
							!recentSet.has(item.value),
					),
				}))
				.filter((group) => group.models.length > 0),
		[groups, searchQuery, recentSet],
	)

	const hasResults =
		filteredFavorites.length > 0 ||
		filteredRecents.length > 0 ||
		filteredGroups.length > 0

	function handleModelChange(next: string) {
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
		void markModelUsed(next)
		setOpen(false)
		setSearchQuery("")
	}

	function renderModelRow(item: ConfigOptionValue) {
		const favorited = favoriteSet.has(item.value)
		const favoritePending = pendingFavoriteIds?.has(item.value) ?? false
		const selected = item.value === value

		return (
			<div
				key={item.value}
				className={cn(
					"group flex items-center gap-0.5 rounded-sm pr-1",
					selected && "bg-white/10",
				)}
			>
				<button
					type="button"
					className="flex min-w-0 flex-1 items-center gap-2 rounded-sm px-2 py-1.5 text-left text-xs outline-none hover:bg-white/10 focus-visible:bg-white/10"
					onClick={() => handleModelChange(item.value)}
				>
					<ProviderIcon
						providerId={providerIdFromGroupOrValue(item.group, item.value)}
					/>
					<span className="min-w-0 flex-1 truncate">
						{modelShortName(item)}
					</span>
					{selected ? (
						<Check className="size-3.5 shrink-0 text-fg" aria-hidden />
					) : (
						<span className="size-3.5 shrink-0" aria-hidden />
					)}
				</button>
				<button
					type="button"
					title={favorited ? "Quitar de favoritos" : "Agregar a favoritos"}
					disabled={favoritePending}
					className="shrink-0 rounded p-1 text-white/35 transition-colors hover:bg-white/5 hover:text-white/70 disabled:opacity-40"
					onPointerDown={(event) => {
						event.preventDefault()
						event.stopPropagation()
					}}
					onClick={(event) => {
						event.preventDefault()
						event.stopPropagation()
						void onToggleFavorite(item.value, !favorited)
					}}
				>
					<Star
						className={cn(
							"size-3.5",
							favorited
								? "fill-white/90 text-white/90"
								: "fill-none text-white/35",
						)}
					/>
				</button>
			</div>
		)
	}

	return (
		<Popover
			open={open}
			onOpenChange={(next) => {
				setOpen(next)
				if (!next) setSearchQuery("")
			}}
		>
			<PopoverTrigger asChild>
				<button
					type="button"
					disabled={promptInFlight}
					aria-label={option.name}
					className={cn(
						"inline-flex h-auto min-w-0 max-w-[14rem] items-center gap-1 rounded-md border-0 bg-transparent px-1.5 py-1 text-[13px] text-white/70 shadow-none transition-colors",
						"hover:bg-white/[0.08] hover:text-white/90 focus-visible:bg-white/[0.08] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-white/15",
						"disabled:cursor-not-allowed disabled:opacity-50",
						open && "bg-white/[0.08] text-white/90",
					)}
				>
					<span className="truncate">
						{value
							? currentModelLabel(value, option.options)
							: option.name}
					</span>
					<ChevronDown className="size-3.5 shrink-0 text-white/55" />
				</button>
			</PopoverTrigger>
			<PopoverContent
				align="start"
				sideOffset={4}
				className="w-[17.5rem] p-0"
				onOpenAutoFocus={(event) => event.preventDefault()}
			>
				<div className="shrink-0 border-b border-border px-2 py-2">
					<div className="relative">
						<Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted" />
						<Input
							type="search"
							value={searchQuery}
							placeholder="Buscar modelo…"
							autoComplete="off"
							className="h-8 pl-8"
							onChange={(event) => setSearchQuery(event.target.value)}
							onKeyDown={(event) => {
								if (event.key === "Escape") {
									event.preventDefault()
									setSearchQuery("")
								}
							}}
						/>
					</div>
				</div>

				<div className="max-h-72 overflow-y-auto overscroll-contain p-1">
					{!hasResults ? (
						<p className="px-3 py-6 text-center text-xs text-muted">
							Sin resultados
						</p>
					) : null}

					{filteredRecents.length > 0 ? (
						<div>
							<div className={GROUP_LABEL_CLASS}>
								<span className="flex items-center gap-1.5">
									<Clock3 className="size-3 text-white/70" />
									Recientes
								</span>
							</div>
							{filteredRecents.map((item) => renderModelRow(item))}
						</div>
					) : null}

					{filteredRecents.length > 0 && filteredFavorites.length > 0 ? (
						<div className="mx-2 my-2 h-px bg-border" />
					) : null}

					{filteredFavorites.length > 0 ? (
						<div>
							<div className={GROUP_LABEL_CLASS}>
								<span className="flex items-center gap-1.5">
									<Star className="size-3 fill-white/70 text-white/70" />
									Favoritos
								</span>
							</div>
							{filteredFavorites.map((item) => renderModelRow(item))}
						</div>
					) : null}

					{filteredFavorites.length > 0 && filteredGroups.length > 0 ? (
						<div className="mx-2 my-2 h-px bg-border" />
					) : null}

					{filteredRecents.length > 0 && filteredGroups.length > 0 ? (
						<div className="mx-2 my-2 h-px bg-border" />
					) : null}

					{filteredGroups.map((group) => (
						<div key={`${group.providerId}-${group.providerLabel}`}>
							<div className={GROUP_LABEL_CLASS}>
								<span className="flex items-center gap-1.5">
									<ProviderIcon providerId={group.providerId} />
									{group.providerLabel}
								</span>
							</div>
							{group.models.map((item) => renderModelRow(item))}
						</div>
					))}
				</div>
			</PopoverContent>
		</Popover>
	)
}
