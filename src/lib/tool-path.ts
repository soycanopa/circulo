import type { ToolCallState } from "@/types/acp"

function asRecord(value: unknown): Record<string, unknown> | null {
	return value && typeof value === "object" ? (value as Record<string, unknown>) : null
}

function asString(value: unknown): string | null {
	return typeof value === "string" && value.trim() ? value.trim() : null
}

export function pickPath(toolCall: ToolCallState): string | undefined {
	if (toolCall.diff?.path) return toolCall.diff.path

	const rawInput = asRecord(toolCall.rawInput) ?? asRecord(toolCall.rawOutput)
	const locations = [rawInput, asRecord(rawInput?.arguments), asRecord(rawInput?.input)]

	for (const record of locations) {
		if (!record) continue
		for (const key of ["path", "file", "filePath", "file_path", "uri"]) {
			const value = asString(record[key])
			if (value) return value
		}
	}

	const title = toolCall.title
	const readMatch = /\b(?:read|write|edit)\s+(.+)$/i.exec(title)
	if (readMatch?.[1]) return readMatch[1].trim()

	return undefined
}