import type {
	CredentialAuthMode,
	CredentialField,
	CredentialRequest,
	CredentialResponseAction,
} from "@/types/acp"

function asRecord(value: unknown): Record<string, unknown> | null {
	return value && typeof value === "object" ? (value as Record<string, unknown>) : null
}

function asString(value: unknown): string | null {
	return typeof value === "string" && value.trim() ? value.trim() : null
}

const MODE_ALIASES: Record<string, CredentialAuthMode> = {
	bearer: "bearer",
	token: "bearer",
	api_key: "bearer",
	apikey: "bearer",
	basic: "basic",
	header: "header",
	"single-header": "header",
	"multi-header": "multi-header",
	multi_header: "multi-header",
	multiheader: "multi-header",
	url: "url",
	oauth: "url",
}

const MODE_LABELS: Record<CredentialAuthMode, string> = {
	bearer: "Token Bearer",
	basic: "Usuario y contraseña",
	header: "Cabecera HTTP",
	"multi-header": "Varias API keys",
	url: "Autorización externa",
}

const DEFAULT_FIELDS: Record<CredentialAuthMode, CredentialField[]> = {
	bearer: [
		{
			key: "token",
			label: "Token / API key",
			placeholder: "sk-…",
			secret: true,
			required: true,
		},
	],
	basic: [
		{ key: "username", label: "Usuario", placeholder: "usuario", required: true },
		{
			key: "password",
			label: "Contraseña",
			placeholder: "••••••••",
			secret: true,
			required: true,
		},
	],
	header: [
		{
			key: "headerName",
			label: "Nombre de cabecera",
			placeholder: "Authorization",
			required: true,
		},
		{
			key: "headerValue",
			label: "Valor",
			placeholder: "Bearer …",
			secret: true,
			required: true,
		},
	],
	"multi-header": [
		{
			key: "primaryKey",
			label: "API key principal",
			placeholder: "sk-…",
			secret: true,
			required: true,
		},
		{
			key: "secondaryKey",
			label: "API key secundaria (opcional)",
			placeholder: "sk-…",
			secret: true,
		},
	],
	url: [],
}

function parseFields(raw: unknown): CredentialField[] {
	if (!Array.isArray(raw)) return []
	const fields: CredentialField[] = []
	for (const entry of raw) {
		const record = asRecord(entry)
		if (!record) continue
		const key = asString(record.key) ?? asString(record.name) ?? asString(record.id)
		const label = asString(record.label) ?? asString(record.title) ?? key
		if (!key || !label) continue
		const placeholder = asString(record.placeholder)
		fields.push({
			key,
			label,
			...(placeholder ? { placeholder } : {}),
			secret: record.secret !== false,
			required: record.required !== false,
		})
	}
	return fields
}

function inferMode(raw: Record<string, unknown>): CredentialAuthMode {
	const explicit =
		asString(raw.mode) ??
		asString(raw.authMode) ??
		asString(raw.auth_mode) ??
		asString(raw.type)
	if (explicit) {
		const normalized = explicit.toLowerCase().replace(/\s+/g, "-")
		return MODE_ALIASES[normalized] ?? "bearer"
	}
	if (asString(raw.url)) return "url"
	return "bearer"
}

/** Normalizes agent/ACP payloads into a credential request the UI can render. */
export function normalizeCredentialRequest(payload: unknown): CredentialRequest | null {
	const root = asRecord(payload)
	if (!root) return null

	const requestId = asString(root.requestId) ?? asString(root.request_id) ?? asString(root.id)
	const sessionId = asString(root.sessionId) ?? asString(root.session_id) ?? ""
	if (!requestId) return null

	const mode = inferMode(root)
	const customFields = parseFields(root.fields ?? root.properties)
	const fields = customFields.length > 0 ? customFields : DEFAULT_FIELDS[mode]

	return {
		requestId,
		sessionId,
		toolCallId: asString(root.toolCallId) ?? asString(root.tool_call_id) ?? undefined,
		title: asString(root.title) ?? asString(root.message) ?? "Credenciales requeridas",
		description: asString(root.description) ?? asString(root.detail) ?? undefined,
		mode,
		fields,
		sourceUrl: asString(root.sourceUrl) ?? asString(root.source_url) ?? undefined,
		url: asString(root.url) ?? undefined,
		serviceName: asString(root.serviceName) ?? asString(root.service) ?? undefined,
	}
}

export function credentialModeLabel(mode: CredentialAuthMode): string {
	return MODE_LABELS[mode]
}

export function emptyCredentialValues(fields: CredentialField[]): Record<string, string> {
	return Object.fromEntries(fields.map((field) => [field.key, ""]))
}

export function validateCredentialValues(
	fields: CredentialField[],
	values: Record<string, string>,
): Record<string, string> {
	const errors: Record<string, string> = {}
	for (const field of fields) {
		if (!field.required) continue
		if (!values[field.key]?.trim()) {
			errors[field.key] = "Campo obligatorio"
		}
	}
	return errors
}

export function maskCredentialValue(value: string): string {
	const trimmed = value.trim()
	if (!trimmed) return "—"
	if (trimmed.length <= 4) return "••••"
	return `${trimmed.slice(0, 2)}${"•".repeat(Math.min(trimmed.length - 4, 12))}${trimmed.slice(-2)}`
}

export function summarizeCredentialSubmission(
	request: CredentialRequest,
	values: Record<string, string>,
	action: CredentialResponseAction,
): string {
	if (action === "cancel") return "Solicitud de credenciales cancelada."
	if (action === "decline") return "Credenciales rechazadas por el usuario."
	if (request.mode === "url") return "Autorización externa aceptada."

	const lines = request.fields
		.filter((field) => values[field.key]?.trim())
		.map((field) => `${field.label}: ${maskCredentialValue(values[field.key])}`)

	return lines.length > 0 ? lines.join("\n") : "Credenciales enviadas."
}