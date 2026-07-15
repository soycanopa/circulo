import type { ConfigOption } from "@/types/acp"

export interface ModelEntry {
	value: string
	name: string
	description?: string
	group: string
}

export interface ModelGroup {
	name: string
	models: ModelEntry[]
}

const PROVIDER_LABELS: Record<string, string> = {
	opencode: "OpenCode Go",
	minimax: "Minimax",
	anthropic: "Anthropic",
	openai: "OpenAI",
	google: "Google",
}

function formatProviderLabel(raw: string): string {
	const key = raw.trim().toLowerCase()
	return PROVIDER_LABELS[key] ?? raw.charAt(0).toUpperCase() + raw.slice(1)
}

function inferGroup(value: string, name: string, group?: string): string {
	if (group?.trim()) return formatProviderLabel(group)
	const slash = value.indexOf("/")
	if (slash > 0) return formatProviderLabel(value.slice(0, slash))
	const colon = value.indexOf(":")
	if (colon > 0) return formatProviderLabel(value.slice(0, colon))
	if (name.includes(" - ")) return formatProviderLabel(name.split(" - ")[0] ?? name)
	return "Otros"
}

export function buildModelGroups(
	options: ConfigOption["options"],
): ModelGroup[] {
	const byGroup = new Map<string, ModelEntry[]>()

	for (const option of options) {
		const group = inferGroup(option.value, option.name, option.group)
		const entry: ModelEntry = {
			value: option.value,
			name: option.name,
			description: option.description,
			group,
		}
		const bucket = byGroup.get(group) ?? []
		bucket.push(entry)
		byGroup.set(group, bucket)
	}

	return [...byGroup.entries()]
		.sort(([a], [b]) => a.localeCompare(b, "es"))
		.map(([name, models]) => ({
			name,
			models: models.sort((a, b) => a.name.localeCompare(b.name, "es")),
		}))
}

export function filterModelGroups(
	groups: ModelGroup[],
	query: string,
	favoriteValues: Set<string>,
): { favorites: ModelEntry[]; groups: ModelGroup[] } {
	const q = query.trim().toLowerCase()

	const allModels = groups.flatMap((group) =>
		group.models.map((model) => ({ ...model, group: group.name })),
	)

	const matches = (model: ModelEntry) =>
		!q ||
		model.name.toLowerCase().includes(q) ||
		model.value.toLowerCase().includes(q) ||
		model.group.toLowerCase().includes(q)

	const favorites = allModels.filter((model) => favoriteValues.has(model.value) && matches(model))

	const filteredGroups = groups
		.map((group) => ({
			...group,
			models: group.models.filter(matches),
		}))
		.filter((group) => group.models.length > 0)

	return { favorites, groups: filteredGroups }
}