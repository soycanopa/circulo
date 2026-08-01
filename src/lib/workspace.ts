/** True when path is a Circulo general chats cwd (legacy or per-space). */
export function isGeneralChatsPath(path: string | null | undefined): boolean {
	if (!path) return false
	if (path.includes("/.circulo/chats")) return true
	// ~/.circulo/spaces/{id}/chats
	return (
		path.includes("/.circulo/spaces/") &&
		(path.endsWith("/chats") || path.includes("/chats/"))
	)
}

/** Short label for a project folder path. */
export function projectLabel(path: string): string {
	if (isGeneralChatsPath(path)) return "Chats"
	const parts = path.split("/").filter(Boolean)
	return parts.slice(-2).join("/") || path
}

/** Last path segment (folder name). */
export function projectName(path: string): string {
	if (isGeneralChatsPath(path)) return "Chats"
	const parts = path.split("/").filter(Boolean)
	return parts[parts.length - 1] ?? path
}
