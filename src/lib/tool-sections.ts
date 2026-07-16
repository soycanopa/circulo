import { getToolGroupKey } from "@/lib/tool-call-groups"
import type { ToolCallState } from "@/types/acp"

export type ToolSectionKey = "files" | "commands" | "other"

export const TOOL_SECTION_ORDER: ToolSectionKey[] = ["files", "commands", "other"]

export const TOOL_SECTION_LABELS: Record<ToolSectionKey, string> = {
	files: "Archivos modificados",
	commands: "Comandos",
	other: "Herramientas",
}

export function isFileEditTool(tool: ToolCallState): boolean {
	if (tool.diff) return true
	const key = getToolGroupKey(tool)
	return key === "edit" || key === "write"
}

export function getToolSection(tool: ToolCallState): ToolSectionKey {
	if (isFileEditTool(tool)) return "files"
	const key = getToolGroupKey(tool)
	if (key === "execute") return "commands"
	return "other"
}

export function partitionToolsBySection(
	toolCalls: ToolCallState[],
): Record<ToolSectionKey, ToolCallState[]> {
	const sections: Record<ToolSectionKey, ToolCallState[]> = {
		files: [],
		commands: [],
		other: [],
	}

	for (const tool of toolCalls) {
		sections[getToolSection(tool)].push(tool)
	}

	return sections
}

export function sectionStatus(
	tools: ToolCallState[],
): ToolCallState["status"] {
	if (tools.some((tool) => tool.status === "failed")) return "failed"
	if (tools.some((tool) => tool.status === "in_progress" || tool.status === "pending")) {
		return "in_progress"
	}
	return "completed"
}