import type { ConfigOption } from "@/types/acp"

export interface ModelEntry {
	value: string
	name: string
	displayName: string
	description?: string
	group: string
	providerId: string
}

export interface ModelGroup {
	name: string
	providerId: string
	models: ModelEntry[]
}

const PROVIDER_LABELS: Record<string, string> = {
	opencode: "OpenCode Go",
	"opencode-go": "OpenCode Go",
	minimax: "Minimax",
	"minimax-coding-plan": "Minimax",
	zai: "Z.ai",
	"zai-coding-plan": "Z.ai",
	xai: "xAI",
	anthropic: "Anthropic",
	openai: "OpenAI",
	google: "Google",
}

export function formatProviderLabel(raw: string): string {
	const key = raw.trim().toLowerCase()
	return PROVIDER_LABELS[key] ?? raw.charAt(0).toUpperCase() + raw.slice(1)
}

function inferProviderId(value: string, name: string, group?: string): string {
	if (group?.trim()) return group.trim().toLowerCase()
	const slash = value.indexOf("/")
	if (slash > 0) return value.slice(0, slash).toLowerCase()
	const colon = value.indexOf(":")
	if (colon > 0) return value.slice(0, colon).toLowerCase()
	if (name.includes(" - ")) return (name.split(" - ")[0] ?? name).trim().toLowerCase()
	return "other"
}

const MODEL_TOKEN_LABELS: Record<string, string> = {
	deepseek: "DeepSeek",
	openai: "OpenAI",
	anthropic: "Anthropic",
	gemini: "Gemini",
	minimax: "MiniMax",
	gpt: "GPT",
	o1: "o1",
	o3: "o3",
	o4: "o4",
	sonnet: "Sonnet",
	haiku: "Haiku",
	opus: "Opus",
	pro: "Pro",
	flash: "Flash",
	mini: "Mini",
}

function formatModelToken(part: string): string {
	const key = part.trim().toLowerCase()
	if (MODEL_TOKEN_LABELS[key]) return MODEL_TOKEN_LABELS[key]
	if (/^v?\d+(?:\.\d+)?[a-z]?$/i.test(part)) return part.toLowerCase()
	return part.charAt(0).toUpperCase() + part.slice(1).toLowerCase()
}

function humanizeModelSlug(slug: string): string {
	return slug
		.split(/[-_/]+/)
		.filter(Boolean)
		.map(formatModelToken)
		.join(" ")
}

/** Canonical model id from ACP option value (e.g. minimax-coding-plan/MiniMax-M2.1). */
function extractModelSlugFromValue(value: string): string | null {
	const trimmed = value.trim()
	if (!trimmed) return null

	const slash = trimmed.indexOf("/")
	if (slash >= 0) {
		const tail = trimmed.slice(slash + 1).trim()
		if (tail) return tail
	}

	const colon = trimmed.indexOf(":")
	if (colon > 0) {
		const tail = trimmed.slice(colon + 1).trim()
		if (tail) return tail
	}

	return null
}

const MODEL_MARKETING_NOISE =
	/\b(?:token\s*plan|coding\s*plan|subscription|pay(?:-|\s)?as(?:-|\s)?you(?:-|\s)?go)\b/gi

function stripModelMarketingNoise(text: string): string {
	return text
		.replace(/\([^)]*(?:token\s*plan|coding\s*plan|context|subscription)[^)]*\)/gi, "")
		.replace(MODEL_MARKETING_NOISE, "")
		.replace(/\s{2,}/g, " ")
		.trim()
}

const MODEL_LIKE_PATTERN =
	/mini-?max|m\d(?:\.\d+)?[a-z]*|gpt-|o\d|claude|gemini|grok|deepseek|sonnet|haiku|opus|llama|qwen/i

function pickModelLikeSegment(text: string): string {
	const parts = text
		.split(/\s*[-–—|·]\s*/)
		.map((part) => part.trim())
		.filter(Boolean)

	const modelPart = parts.find((part) => MODEL_LIKE_PATTERN.test(part))
	if (modelPart) return modelPart

	return parts[parts.length - 1] ?? text
}

function stripProviderPrefix(name: string, providerId: string): string {
	const label = formatProviderLabel(providerId)
	const patterns = [
		new RegExp(`^${label}\\s*[-–—:|·]\\s*`, "i"),
		new RegExp(`^${providerId.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*[-–—:/|·]\\s*`, "i"),
		new RegExp(`^${label}\\s+`, "i"),
	]

	let result = name
	for (const pattern of patterns) {
		const stripped = result.replace(pattern, "").trim()
		if (stripped && stripped !== result) {
			result = stripped
		}
	}
	return result
}

export function modelDisplayName(name: string, value: string, providerId: string): string {
	const fromValue = extractModelSlugFromValue(value)
	if (fromValue) return humanizeModelSlug(fromValue)

	let cleaned = stripModelMarketingNoise(name)
	cleaned = stripProviderPrefix(cleaned, providerId)
	cleaned = stripModelMarketingNoise(cleaned)

	if (cleaned.includes(" - ")) {
		cleaned = pickModelLikeSegment(cleaned.replace(/\s+-\s+/g, " - "))
	} else {
		cleaned = pickModelLikeSegment(cleaned)
	}

	return humanizeModelSlug(cleaned || name.trim())
}

export function buildModelGroups(options: ConfigOption["options"]): ModelGroup[] {
	const byGroup = new Map<string, ModelEntry[]>()
	const providerIds = new Map<string, string>()

	for (const option of options) {
		const providerId = inferProviderId(option.value, option.name, option.group)
		const group = formatProviderLabel(providerId)
		const entry: ModelEntry = {
			value: option.value,
			name: option.name,
			displayName: modelDisplayName(option.name, option.value, providerId),
			description: option.description,
			group,
			providerId,
		}
		const bucket = byGroup.get(group) ?? []
		bucket.push(entry)
		byGroup.set(group, bucket)
		providerIds.set(group, providerId)
	}

	return [...byGroup.entries()]
		.sort(([a], [b]) => a.localeCompare(b, "es"))
		.map(([name, models]) => ({
			name,
			providerId: providerIds.get(name) ?? name.toLowerCase(),
			models: models.sort((a, b) => a.displayName.localeCompare(b.displayName, "es")),
		}))
}

export function filterModelGroups(
	groups: ModelGroup[],
	query: string,
	favoriteValues: Set<string>,
): { favorites: ModelEntry[]; groups: ModelGroup[] } {
	const q = query.trim().toLowerCase()

	const allModels = groups.flatMap((group) => group.models)

	const matches = (model: ModelEntry) =>
		!q ||
		model.displayName.toLowerCase().includes(q) ||
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

export function findModelEntry(
	groups: ModelGroup[],
	value: string,
): ModelEntry | null {
	for (const group of groups) {
		const match = group.models.find((model) => model.value === value)
		if (match) return match
	}
	return null
}

/** Short label for profile/history when only the stored model id is available. */
export function formatLastModelLabel(value: string | null | undefined): string {
	if (!value?.trim()) return "—"
	const slug = extractModelSlugFromValue(value) ?? value.trim()
	return humanizeModelSlug(slug)
}