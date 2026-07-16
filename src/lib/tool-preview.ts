import { getToolGroupKey } from "@/lib/tool-call-groups"
import { normalizeHighlightLanguage } from "@/lib/shiki-cache"
import type { ToolCallState } from "@/types/acp"

export type ToolPreviewKind = "code" | "diff" | "terminal" | "text"

export interface ToolPreviewModel {
	kind: ToolPreviewKind
	title: string
	badge: string
	path?: string
	language: string
	lineRange?: string
	code: string
	diff?: ToolCallState["diff"]
}

export interface MultiDiffEntry {
	id: string
	title: string
	badge: string
	path: string
	oldText?: string
	newText: string
}

function asRecord(value: unknown): Record<string, unknown> | null {
	return value && typeof value === "object" ? (value as Record<string, unknown>) : null
}

function asString(value: unknown): string | null {
	return typeof value === "string" && value.trim() ? value.trim() : null
}

const EXTENSION_LANGUAGE: Record<string, string> = {
	ts: "typescript",
	tsx: "tsx",
	js: "javascript",
	jsx: "jsx",
	rs: "rust",
	go: "go",
	py: "python",
	rb: "ruby",
	json: "json",
	md: "markdown",
	yml: "yaml",
	yaml: "yaml",
	toml: "toml",
	css: "css",
	html: "html",
	sh: "bash",
	zsh: "bash",
}

function languageFromPath(path: string | undefined): string {
	if (!path) return "text"
	const ext = path.split(".").pop()?.toLowerCase()
	return normalizeHighlightLanguage(ext ? (EXTENSION_LANGUAGE[ext] ?? ext) : "text")
}

function pickPath(toolCall: ToolCallState): string | undefined {
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

function pickLineRange(toolCall: ToolCallState): string | undefined {
	const haystack = `${toolCall.title}\n${toolCall.content}`
	const patterns = [
		/\bL(\d+)\s*[-–]\s*L?(\d+)\b/i,
		/\blines?\s+(\d+)\s*[-–]\s*(\d+)\b/i,
		/\b(\d+):(\d+)\b/,
	]

	for (const pattern of patterns) {
		const match = pattern.exec(haystack)
		if (!match) continue
		return `L${match[1]}–${match[2]}`
	}

	return undefined
}

export function previewBadge(toolCall: ToolCallState): string {
	const group = getToolGroupKey(toolCall)
	const labels: Record<string, string> = {
		read: "Read",
		write: "Write",
		edit: "Edit",
		execute: "Terminal",
		search: "Search",
		websearch: "Web",
		fetch: "Fetch",
	}
	return labels[group] ?? (toolCall.kind ?? "Tool")
}

export function canExpandTool(toolCall: ToolCallState): boolean {
	if (toolCall.diff) return true
	if (toolCall.content.trim()) return true
	return false
}

export function buildToolPreview(toolCall: ToolCallState): ToolPreviewModel | null {
	if (!canExpandTool(toolCall)) return null

	const path = pickPath(toolCall)
	const badge = previewBadge(toolCall)
	const lineRange = pickLineRange(toolCall)

	if (toolCall.diff) {
		return {
			kind: "diff",
			title: path ?? toolCall.title,
			badge,
			path,
			language: languageFromPath(path),
			lineRange,
			code: toolCall.content,
			diff: toolCall.diff,
		}
	}

	const group = getToolGroupKey(toolCall)
	const kind: ToolPreviewKind =
		group === "execute" ? "terminal" : group === "read" ? "code" : "text"

	return {
		kind,
		title: path ?? toolCall.title,
		badge,
		path,
		language: languageFromPath(path),
		lineRange,
		code: toolCall.content,
	}
}

export function collectDiffTools(toolCalls: ToolCallState[]): MultiDiffEntry[] {
	return toolCalls
		.filter((tool) => tool.diff)
		.map((tool) => ({
			id: tool.id,
			title: tool.title,
			badge: previewBadge(tool),
			path: tool.diff!.path,
			oldText: tool.diff!.oldText,
			newText: tool.diff!.newText,
		}))
}

export function hasMultiDiffTools(toolCalls: ToolCallState[]): boolean {
	return collectDiffTools(toolCalls).length >= 2
}