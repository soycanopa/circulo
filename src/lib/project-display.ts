import { GENERAL_CHAT_PROJECT } from "@/lib/preferences"

export function getProjectDisplayName(projectPath: string | null): string {
	if (!projectPath) return "Proyecto"
	if (projectPath === GENERAL_CHAT_PROJECT) return "Chats"
	return projectPath.split("/").pop() ?? "Proyecto"
}