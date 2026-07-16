import { invoke } from "@tauri-apps/api/core"

export interface SkillsShSearchResult {
	id: string
	skillId: string
	name: string
	installs: number
	source: string
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

	return invoke<SkillsShSearchResult[]>("search_skills_sh", { query: trimmed })
}