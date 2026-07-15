import { GENERAL_CHAT_PROJECT } from "@/lib/preferences"

export function isGeneralChatProject(projectPath: string | null): boolean {
	return projectPath === GENERAL_CHAT_PROJECT
}

export function getProjectDisplayName(projectPath: string | null): string {
	if (!projectPath) return "Chats"
	if (isGeneralChatProject(projectPath)) return "Chats"
	return projectPath.split("/").pop() ?? "Proyecto"
}

/** Last path segment for session title prefix (e.g. soycanopa/Sesión 1). */
export function getProjectDirectoryLabel(projectPath: string | null): string {
	if (!projectPath) return "…"
	return projectPath.split("/").pop() ?? "…"
}