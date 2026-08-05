import {
	providerIdFromGroupOrValue,
	providerLabelFromGroupOrValue,
	resolveProvider,
} from "@/lib/provider-registry"
import type { ConfigOptionValue } from "@/types/acp"

export interface ModelGroup {
	providerId: string
	providerLabel: string
	models: ConfigOptionValue[]
}

export interface GroupedModelOptions {
	favorites: ConfigOptionValue[]
	groups: ModelGroup[]
}

/** Models in the order the user last selected them (from `recentModelIds`). */
export function extractRecentModels(
	options: ConfigOptionValue[],
	recentIds: string[],
): ConfigOptionValue[] {
	const byValue = new Map(options.map((item) => [item.value, item]))
	const recents: ConfigOptionValue[] = []
	for (const id of recentIds) {
		const hit = byValue.get(id)
		if (hit) recents.push(hit)
	}
	return recents
}

function groupKey(item: ConfigOptionValue): string {
	const providerId = providerIdFromGroupOrValue(item.group, item.value)
	const label = providerLabelFromGroupOrValue(item.group, item.value)
	return `${providerId}\0${label}`
}

export function groupModelOptions(
	options: ConfigOptionValue[],
	favoriteIds: string[],
): GroupedModelOptions {
	const favoriteSet = new Set(favoriteIds)
	const favorites: ConfigOptionValue[] = []
	const groupMap = new Map<string, ModelGroup>()

	for (const item of options) {
		if (favoriteSet.has(item.value)) {
			favorites.push(item)
			continue
		}

		const providerId = providerIdFromGroupOrValue(item.group, item.value)
		const providerLabel = providerLabelFromGroupOrValue(item.group, item.value)
		const key = groupKey(item)
		const existing = groupMap.get(key)
		if (existing) {
			existing.models.push(item)
		} else {
			groupMap.set(key, {
				providerId,
				providerLabel,
				models: [item],
			})
		}
	}

	const groups = [...groupMap.values()].sort((a, b) =>
		a.providerLabel.localeCompare(b.providerLabel),
	)

	const favoriteOrder = new Map(favoriteIds.map((id, index) => [id, index]))
	favorites.sort(
		(a, b) =>
			(favoriteOrder.get(a.value) ?? 0) - (favoriteOrder.get(b.value) ?? 0),
	)

	return { favorites, groups }
}

export function modelShortName(item: ConfigOptionValue): string {
	const fromValue = () => {
		const slash = item.value.lastIndexOf("/")
		return slash !== -1 ? item.value.slice(slash + 1) : item.value
	}

	const name = item.name?.trim()
	if (!name) return fromValue()

	const spacedSep = name.lastIndexOf(" / ")
	if (spacedSep !== -1) return name.slice(spacedSep + 3).trim()

	const slash = name.indexOf("/")
	if (slash !== -1) return name.slice(slash + 1).trim()

	return name
}

export function modelDisplayName(item: ConfigOptionValue): string {
	if (item.name) return item.name
	const provider = resolveProvider(
		providerIdFromGroupOrValue(item.group, item.value),
	)
	const suffix = item.value.includes("/")
		? item.value.slice(item.value.indexOf("/") + 1)
		: item.value
	return `${provider.label} / ${suffix}`
}

export function modelMatchesQuery(
	item: ConfigOptionValue,
	query: string,
): boolean {
	const q = query.trim().toLowerCase()
	if (!q) return true
	const name = item.name ?? ""
	const group = item.group ?? ""
	return (
		item.value.toLowerCase().includes(q) ||
		name.toLowerCase().includes(q) ||
		group.toLowerCase().includes(q) ||
		modelDisplayName(item).toLowerCase().includes(q)
	)
}

export function currentModelLabel(
	currentValue: string,
	options: ConfigOptionValue[],
): string {
	const hit = options.find((item) => item.value === currentValue)
	if (!hit) return currentValue
	return modelShortName(hit)
}
