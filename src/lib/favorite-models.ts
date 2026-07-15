const FAVORITE_MODELS_KEY = "forge-favorite-models"

export function getFavoriteModels(): string[] {
	try {
		const raw = localStorage.getItem(FAVORITE_MODELS_KEY)
		if (!raw) return []
		const parsed = JSON.parse(raw) as unknown
		return Array.isArray(parsed) ? parsed.filter((v) => typeof v === "string") : []
	} catch {
		return []
	}
}

export function isFavoriteModel(value: string): boolean {
	return getFavoriteModels().includes(value)
}

export function toggleFavoriteModel(value: string): string[] {
	const current = getFavoriteModels()
	const next = current.includes(value)
		? current.filter((entry) => entry !== value)
		: [...current, value]
	localStorage.setItem(FAVORITE_MODELS_KEY, JSON.stringify(next))
	return next
}