import { useAtom } from "jotai"
import { ChevronDown, Star } from "lucide-react"
import { useMemo, useRef, useState } from "react"
import { SelectorMenuItem } from "@/components/chat/selector-menu-item"
import { useDismissOnOutside } from "@/hooks/use-dismiss-on-outside"
import { InputGroupButton } from "@/components/ui/input-group"
import {
	getFavoriteModels,
	isFavoriteModel,
	toggleFavoriteModel,
} from "@/lib/favorite-models"
import { buildModelGroups, filterModelGroups } from "@/lib/model-groups"
import { setLastModel } from "@/lib/preferences"
import { setConfigOption } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import { configOptionsAtom } from "@/stores/atoms"

export function ModelSelector() {
	const [configOptions, setConfigOptions] = useAtom(configOptionsAtom)
	const [open, setOpen] = useState(false)
	const [query, setQuery] = useState("")
	const [favorites, setFavorites] = useState(getFavoriteModels)
	const rootRef = useRef<HTMLDivElement>(null)

	useDismissOnOutside(rootRef, () => setOpen(false), open)

	const modelOption = configOptions.find(
		(option) => option.category?.toLowerCase().includes("model") || option.id === "model",
	)

	const currentName = useMemo(() => {
		if (!modelOption) return "Modelo"
		return (
			modelOption.options.find((o) => o.value === modelOption.currentValue)?.name ??
			modelOption.currentValue
		)
	}, [modelOption])

	const modelGroups = useMemo(
		() => (modelOption ? buildModelGroups(modelOption.options) : []),
		[modelOption],
	)

	const favoriteSet = useMemo(() => new Set(favorites), [favorites])

	const { favorites: favoriteModels, groups: visibleGroups } = useMemo(
		() => filterModelGroups(modelGroups, query, favoriteSet),
		[modelGroups, query, favoriteSet],
	)

	const hasModels = Boolean(modelOption && modelOption.options.length > 0)
	const hasVisible =
		favoriteModels.length > 0 || visibleGroups.some((group) => group.models.length > 0)

	async function handleSelect(value: string) {
		if (!modelOption) return
		await setConfigOption(modelOption.id, value)
		setLastModel(value)
		setConfigOptions((current) =>
			current.map((option) =>
				option.id === modelOption.id ? { ...option, currentValue: value } : option,
			),
		)
		setOpen(false)
		setQuery("")
	}

	function handleToggleFavorite(event: React.MouseEvent, value: string) {
		event.stopPropagation()
		event.preventDefault()
		setFavorites(toggleFavoriteModel(value))
	}

	return (
		<div ref={rootRef} className="relative">
			<InputGroupButton
				variant="ghost"
				size="sm"
				className="h-7 max-w-48 gap-1 px-2 text-xs"
				disabled={!hasModels}
				onClick={() => hasModels && setOpen((v) => !v)}
			>
				<span className="truncate">{currentName}</span>
				<ChevronDown className="size-3 shrink-0 opacity-60" />
			</InputGroupButton>

			{open && modelOption ? (
				<div className="absolute bottom-full left-0 z-30 mb-2 w-72 overflow-hidden rounded-lg border border-border bg-popover shadow-lg">
					<div className="border-b border-border p-2">
						<input
							value={query}
							onChange={(e) => setQuery(e.target.value)}
							placeholder="Buscar modelo…"
							className="h-7 w-full rounded-md border border-input bg-muted/40 px-2 text-xs outline-none focus:ring-1 focus:ring-ring/40"
							autoFocus
						/>
					</div>
					<div className="scrollbar-thin max-h-64 overflow-y-auto p-1">
						{!hasVisible ? (
							<p className="px-2 py-2 text-xs text-muted-foreground">Sin resultados</p>
						) : (
							<>
								{favoriteModels.length > 0 ? (
									<section className="mb-1">
										<p className="px-2 py-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
											Favoritos
										</p>
										<ul>
											{favoriteModels.map((model) => (
												<li key={`fav-${model.value}`}>
													<ModelRow
														model={model}
														isActive={model.value === modelOption.currentValue}
														isFavorite
														onSelect={() => void handleSelect(model.value)}
														onToggleFavorite={(e) => handleToggleFavorite(e, model.value)}
													/>
												</li>
											))}
										</ul>
									</section>
								) : null}

								{visibleGroups.map((group) => (
									<section key={group.name} className="mb-1 last:mb-0">
										<p className="px-2 py-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
											{group.name}
										</p>
										<ul>
											{group.models.map((model) => (
												<li key={model.value}>
													<ModelRow
														model={model}
														isActive={model.value === modelOption.currentValue}
														isFavorite={isFavoriteModel(model.value)}
														onSelect={() => void handleSelect(model.value)}
														onToggleFavorite={(e) => handleToggleFavorite(e, model.value)}
													/>
												</li>
											))}
										</ul>
									</section>
								))}
							</>
						)}
					</div>
				</div>
			) : null}
		</div>
	)
}

function ModelRow({
	model,
	isActive,
	isFavorite,
	onSelect,
	onToggleFavorite,
}: {
	model: { value: string; name: string; description?: string }
	isActive: boolean
	isFavorite: boolean
	onSelect: () => void
	onToggleFavorite: (event: React.MouseEvent) => void
}) {
	return (
		<div className="group/model-row relative">
			<SelectorMenuItem active={isActive} onClick={onSelect} className="pr-8">
				<span className="truncate">{model.name}</span>
			</SelectorMenuItem>
			<button
				type="button"
				title={isFavorite ? "Quitar de favoritos" : "Agregar a favoritos"}
				onClick={onToggleFavorite}
				className={cn(
					"absolute right-1 top-1/2 flex size-6 -translate-y-1/2 items-center justify-center rounded-md text-muted-foreground opacity-0 transition-opacity hover:bg-white/10 hover:text-foreground group-hover/model-row:opacity-100",
					isFavorite && "text-amber-400 opacity-100",
				)}
			>
				<Star className={cn("size-3", isFavorite && "fill-current")} />
			</button>
		</div>
	)
}