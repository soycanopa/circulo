import type { ToolCallState } from "@/types/acp"

export interface ToolCallGroup {
	key: string
	label: string
	tools: ToolCallState[]
}

const GROUP_LABELS: Record<string, string> = {
	websearch: "Web Search",
	search: "Búsqueda",
	read: "Lectura",
	write: "Escritura",
	edit: "Edición",
	execute: "Terminal",
	fetch: "Fetch",
	other: "Herramientas",
}

export function getToolGroupKey(tool: ToolCallState): string {
	const kind = tool.kind?.toLowerCase() ?? ""
	const title = tool.title.toLowerCase()

	if (
		kind === "websearch" ||
		kind === "web_search" ||
		title.includes("web search") ||
		title.includes("exa")
	) {
		return "websearch"
	}
	if (kind === "search" || title.includes("search") || title.includes("grep")) {
		return "search"
	}
	if (kind === "read" || title.includes("read")) return "read"
	if (kind === "write" || kind === "edit" || title.includes("write") || title.includes("edit")) {
		return kind === "write" ? "write" : "edit"
	}
	if (kind === "execute" || title.includes("bash") || title.includes("command")) return "execute"
	if (kind === "fetch") return "fetch"
	if (kind) return kind
	return "other"
}

export function groupToolCalls(toolCalls: ToolCallState[]): ToolCallGroup[] {
	const order: string[] = []
	const map = new Map<string, ToolCallState[]>()

	for (const tool of toolCalls) {
		const key = getToolGroupKey(tool)
		if (!map.has(key)) {
			map.set(key, [])
			order.push(key)
		}
		map.get(key)!.push(tool)
	}

	return order.map((key) => ({
		key,
		label: GROUP_LABELS[key] ?? key,
		tools: map.get(key)!,
	}))
}

export function groupStatus(tools: ToolCallState[]): ToolCallState["status"] {
	if (tools.some((t) => t.status === "failed")) return "failed"
	if (tools.some((t) => t.status === "in_progress" || t.status === "pending")) {
		return "in_progress"
	}
	return "completed"
}