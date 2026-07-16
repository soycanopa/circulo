export interface SkillsShSearchResult {
	id: string
	skillId: string
	name: string
	installs: number
	source: string
}

interface SkillsShSearchResponse {
	query: string
	searchType: string
	skills: SkillsShSearchResult[]
}

export function buildSkillsShPackage(result: SkillsShSearchResult): string {
	return `${result.source}@${result.skillId}`
}

export function buildSkillsShUrl(result: SkillsShSearchResult): string {
	return `https://skills.sh/${result.id}`
}

export function formatSkillsShInstalls(count: number): string {
	if (count >= 1_000_000) {
		return `${(count / 1_000_000).toFixed(1).replace(/\.0$/, "")}M`
	}
	if (count >= 1_000) {
		return `${(count / 1_000).toFixed(1).replace(/\.0$/, "")}K`
	}
	return count.toLocaleString()
}

export async function searchSkillsSh(query: string): Promise<SkillsShSearchResult[]> {
	const trimmed = query.trim()
	if (!trimmed) return []

	const response = await fetch(
		`https://skills.sh/api/search?q=${encodeURIComponent(trimmed)}`,
	)
	if (!response.ok) {
		throw new Error(`skills.sh respondió con ${response.status}`)
	}

	const data = (await response.json()) as SkillsShSearchResponse
	return data.skills ?? []
}