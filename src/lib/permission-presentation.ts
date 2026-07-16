import type { PermissionOption, PermissionRequest } from "@/types/acp"

function asRecord(value: unknown): Record<string, unknown> | null {
	return value && typeof value === "object" ? (value as Record<string, unknown>) : null
}

function asString(value: unknown): string | null {
	return typeof value === "string" && value.trim() ? value.trim() : null
}

export interface PermissionPresentation {
	toolLabel: string
	kind: string | null
	summary: string
	detail: string | null
}

const KIND_LABELS: Record<string, string> = {
	read: "Leer archivo",
	write: "Escribir archivo",
	edit: "Editar archivo",
	execute: "Ejecutar comando",
	search: "Buscar",
	fetch: "Obtener URL",
}

const OPTION_LABELS: Record<string, string> = {
	allow_once: "Aprobar",
	allow_always: "Siempre permitir",
	reject_once: "Denegar",
	reject_always: "Siempre denegar",
}

const OPTION_ORDER = ["allow_once", "allow_always", "reject_once", "reject_always"] as const

function pickCommand(toolCall: Record<string, unknown>): string | null {
	const rawInput = asRecord(toolCall.rawInput) ?? asRecord(toolCall.input)
	const locations = [
		toolCall,
		rawInput,
		asRecord(toolCall.arguments),
		asRecord(rawInput?.arguments),
	]

	for (const record of locations) {
		if (!record) continue
		for (const key of ["command", "cmd", "script", "shell"]) {
			const value = asString(record[key])
			if (value) return value
		}
	}

	return null
}

function pickPath(toolCall: Record<string, unknown>): string | null {
	const rawInput = asRecord(toolCall.rawInput) ?? asRecord(toolCall.input)
	const locations = [toolCall, rawInput, asRecord(toolCall.arguments), asRecord(rawInput?.arguments)]

	for (const record of locations) {
		if (!record) continue
		for (const key of ["path", "file", "filePath", "file_path", "uri"]) {
			const value = asString(record[key])
			if (value) return value
		}
	}

	return null
}

export function presentPermissionRequest(request: PermissionRequest): PermissionPresentation {
	const toolCall = request.toolCall
	const kind = asString(toolCall.kind)?.toLowerCase() ?? null
	const title = asString(toolCall.title)
	const command = pickCommand(toolCall)
	const path = pickPath(toolCall)

	const toolLabel =
		title ??
		(kind ? (KIND_LABELS[kind] ?? kind) : null) ??
		asString(toolCall.name) ??
		"Herramienta del agente"

	const summary =
		command ??
		path ??
		asString(toolCall.description) ??
		"El agente necesita tu confirmación para continuar."

	const detail =
		command && path
			? path
			: command && title && title !== summary
				? title
				: path && title && title !== summary
					? title
					: null

	return {
		toolLabel,
		kind,
		summary,
		detail,
	}
}

export function labelPermissionOption(option: PermissionOption): string {
	const normalized = option.kind.trim().toLowerCase()
	return (
		OPTION_LABELS[normalized as keyof typeof OPTION_LABELS] ??
		option.name?.trim() ??
		option.kind
	)
}

export function sortPermissionOptions(options: PermissionOption[]): PermissionOption[] {
	return [...options].sort((a, b) => {
		const aIndex = OPTION_ORDER.indexOf(a.kind as (typeof OPTION_ORDER)[number])
		const bIndex = OPTION_ORDER.indexOf(b.kind as (typeof OPTION_ORDER)[number])
		const safeA = aIndex === -1 ? OPTION_ORDER.length : aIndex
		const safeB = bIndex === -1 ? OPTION_ORDER.length : bIndex
		return safeA - safeB
	})
}

export function permissionKindLabel(kind: string | null): string | null {
	if (!kind) return null
	return KIND_LABELS[kind] ?? kind
}