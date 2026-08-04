export interface ProviderInfo {
	id: string
	label: string
}

const PROVIDERS: Record<string, ProviderInfo> = {
	opencode: { id: "opencode", label: "OpenCode Zen" },
	minimax: { id: "minimax", label: "MiniMax" },
	anthropic: { id: "anthropic", label: "Anthropic" },
	openai: { id: "openai", label: "OpenAI" },
	google: { id: "google", label: "Google" },
	grok: { id: "grok", label: "Grok" },
	cursor: { id: "cursor", label: "Cursor" },
	deepseek: { id: "deepseek", label: "DeepSeek" },
	mistral: { id: "mistral", label: "Mistral" },
	groq: { id: "groq", label: "Groq" },
}

export function providerIdFromModelValue(value: string): string {
	const slash = value.indexOf("/")
	if (slash <= 0) return value
	return value.slice(0, slash)
}

export function normalizeProviderId(providerId: string): string {
	const key = providerId.toLowerCase()
	if (key.startsWith("minimax")) return "minimax"
	return key
}

export function resolveProvider(providerId: string): ProviderInfo {
	const normalized = normalizeProviderId(providerId)
	const hit = PROVIDERS[normalized]
	if (hit) return hit
	const label = providerId
		.split(/[-_]/)
		.filter(Boolean)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
		.join(" ")
	return { id: normalized, label: label || providerId }
}

export function providerIdFromGroupOrValue(
	group: string | null | undefined,
	value: string,
): string {
	if (group) {
		const fromGroup = Object.values(PROVIDERS).find(
			(p) => p.label.toLowerCase() === group.toLowerCase(),
		)
		if (fromGroup) return fromGroup.id
		return normalizeProviderId(group)
	}
	return normalizeProviderId(providerIdFromModelValue(value))
}

export function providerLabelFromGroupOrValue(
	group: string | null | undefined,
	value: string,
): string {
	if (group) return group
	return resolveProvider(providerIdFromModelValue(value)).label
}
