import { getChatsProjectPath } from "@/lib/app-settings"
import { isGeneralChatProject } from "@/lib/project-display"

const RECENT_PROJECTS_KEY = "circulo-recent-projects"
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
	if (!path || path === getChatsProjectPath()) return
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

/** Project folders shown in the sidebar (recent + currently open, excluding Chats). */
export function getActiveProjectPaths(projectPath: string | null): string[] {
	const recent = getRecentProjects()
	if (
		projectPath &&
		!isGeneralChatProject(projectPath) &&
		!recent.includes(projectPath)
	) {
		return [projectPath, ...recent]
	}
	return recent
}

export function countActiveProjects(projectPath: string | null): number {
	return getActiveProjectPaths(projectPath).length
}

export function filterRecentProjects(projects: string[], query: string): string[] {
	const normalized = query.trim().toLowerCase()
	if (!normalized) return projects
	return projects.filter((path) => {
		const label = getRecentProjectLabel(path).toLowerCase()
		return label.includes(normalized) || path.toLowerCase().includes(normalized)
	})
}