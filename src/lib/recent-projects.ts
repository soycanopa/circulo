import { GENERAL_CHAT_PROJECT } from "@/lib/preferences"

const RECENT_PROJECTS_KEY = "forge-recent-projects"
const MAX_STORED_RECENT_PROJECTS = 20

export const MAX_RECENT_PROJECTS_DISPLAY = 5

export function getRecentProjects(): string[] {
	try {
		const raw = localStorage.getItem(RECENT_PROJECTS_KEY)
		if (!raw) return []
		const parsed: unknown = JSON.parse(raw)
		if (!Array.isArray(parsed)) return []
		return parsed.filter((entry): entry is string => typeof entry === "string")
	} catch {
		return []
	}
}

export function addRecentProject(path: string): void {
	if (!path || path === GENERAL_CHAT_PROJECT) return
	const next = [path, ...getRecentProjects().filter((entry) => entry !== path)].slice(
		0,
		MAX_STORED_RECENT_PROJECTS,
	)
	localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(next))
}

export function removeRecentProject(path: string): void {
	if (!path) return
	const next = getRecentProjects().filter((entry) => entry !== path)
	localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(next))
}

export function getRecentProjectLabel(path: string): string {
	return path.split("/").pop() ?? path
}

export function filterRecentProjects(projects: string[], query: string): string[] {
	const normalized = query.trim().toLowerCase()
	if (!normalized) return projects
	return projects.filter((path) => {
		const label = getRecentProjectLabel(path).toLowerCase()
		return label.includes(normalized) || path.toLowerCase().includes(normalized)
	})
}