import type { ConfigOption } from "@/types/acp"

export type ContextBreakdownId =
	| "mcpTools"
	| "systemTools"
	| "skills"
	| "systemPrompt"
	| "metaContext"
	| "messages"

export interface ContextBreakdownItem {
	id: ContextBreakdownId
	label: string
	percent: number
}

export interface ContextWindowSnapshot {
	usedTokens: number
	maxTokens: number | null
	usedPercent: number | null
	costUsd: number | null
	breakdown: ContextBreakdownItem[]
	updatedAt: number
}

const BREAKDOWN_LABELS: Record<ContextBreakdownId, string> = {
	mcpTools: "MCP tools",
	systemTools: "System tools",
	skills: "Skills",
	systemPrompt: "System prompt",
	metaContext: "Meta context",
	messages: "Messages",
}

const BREAKDOWN_ORDER: ContextBreakdownId[] = [
	"mcpTools",
	"systemTools",
	"skills",
	"systemPrompt",
	"metaContext",
	"messages",
]

function asRecord(value: unknown): Record<string, unknown> | null {
	return value && typeof value === "object" ? (value as Record<string, unknown>) : null
}

function asFiniteNumber(value: unknown): number | null {
	return typeof value === "number" && Number.isFinite(value) ? value : null
}

function clampPercent(value: number): number {
	return Math.max(0, Math.min(100, value))
}

export function formatContextTokens(value: number | null | undefined): string {
	if (value == null || !Number.isFinite(value)) return "0"
	if (value < 1_000) return `${Math.round(value)}`
	if (value < 10_000) return `${(value / 1_000).toFixed(1).replace(/\.0$/, "")}K`
	if (value < 1_000_000) return `${Math.round(value / 1_000)}K`
	return `${(value / 1_000_000).toFixed(1).replace(/\.0$/, "")}M`
}

export function formatCostUsd(value: number | null | undefined): string {
	if (value == null || !Number.isFinite(value)) return "—"
	if (value < 0.0001) return `$${value.toFixed(6)}`
	if (value < 0.001) return `$${value.toFixed(5)}`
	if (value < 0.01) return `$${value.toFixed(4)}`
	if (value < 0.1) return `$${value.toFixed(3)}`
	return `$${value.toFixed(2)}`
}

function parseBreakdownPercent(value: unknown): number | null {
	const direct = asFiniteNumber(value)
	if (direct !== null) return clampPercent(direct)
	const record = asRecord(value)
	if (!record) return null
	const percent = asFiniteNumber(record.percent ?? record.percentage ?? record.share)
	return percent === null ? null : clampPercent(percent)
}

function parseBreakdownItems(source: unknown): ContextBreakdownItem[] {
	const record = asRecord(source)
	if (!record) return []

	if (Array.isArray(source)) {
		return source.flatMap((entry) => {
			const item = asRecord(entry)
			if (!item) return []
			const id = typeof item.id === "string" ? (item.id as ContextBreakdownId) : null
			const label = typeof item.label === "string" ? item.label : typeof item.name === "string" ? item.name : null
			const percent = parseBreakdownPercent(item.percent ?? item.percentage ?? item.share)
			if (!label || percent === null) return []
			return [{ id: id ?? "messages", label, percent }]
		})
	}

	const items: ContextBreakdownItem[] = []
	for (const id of BREAKDOWN_ORDER) {
		const percent = parseBreakdownPercent(record[id])
		if (percent === null) continue
		items.push({ id, label: BREAKDOWN_LABELS[id], percent })
	}
	return items
}

function findBreakdownSource(update: Record<string, unknown>): unknown {
	const context = asRecord(update.context)
	return (
		update.breakdown ??
		update.contextBreakdown ??
		update.context_breakdown ??
		update.windows ??
		context?.breakdown ??
		context?.windows
	)
}

/**
 * Parse OpenCode ACP `usage_update` (totals + optional extended breakdown if the agent sends them).
 *
 * Note: OpenCode's per-category breakdown (MCP tools, Skills, etc.) is not in `usage_update` today;
 * a future fallback may fetch `/session/:id/context` (HTTP), SDK, or CLI — see docs/ACP.md § Límites.
 */
export function parseUsageUpdate(payload: unknown): ContextWindowSnapshot | null {
	const root = asRecord(payload)
	const update = asRecord(root?.update)
	if (!update || update.sessionUpdate !== "usage_update") return null

	const used =
		asFiniteNumber(update.used) ??
		asFiniteNumber(update.usedTokens) ??
		asFiniteNumber(update.inputTokens) ??
		0
	const maxTokens =
		asFiniteNumber(update.size) ??
		asFiniteNumber(update.maxTokens) ??
		asFiniteNumber(update.contextWindow) ??
		asFiniteNumber(update.limit) ??
		null

	const explicitPercent = asFiniteNumber(update.usedPercent ?? update.percent)
	const usedPercent =
		explicitPercent !== null
			? clampPercent(explicitPercent)
			: maxTokens && maxTokens > 0
				? clampPercent((used / maxTokens) * 100)
				: null

	const costRecord = asRecord(update.cost)
	const costUsd = asFiniteNumber(costRecord?.amount ?? update.costUsd)

	const breakdown = parseBreakdownItems(findBreakdownSource(update))

	return {
		usedTokens: used,
		maxTokens,
		usedPercent,
		costUsd,
		breakdown,
		updatedAt: Date.now(),
	}
}

export function deriveMeterPercent(snapshot: ContextWindowSnapshot): number {
	return clampPercent(snapshot.usedPercent ?? 0)
}

function findModelOption(configOptions: ConfigOption[]) {
	return configOptions.find(
		(option) => option.id === "model" || option.category?.toLowerCase().includes("model"),
	)
}

function parseTokenLimitFromText(text: string): number | null {
	const normalized = text.replace(/,/g, " ")

	const millionMatch = /(\d+(?:\.\d+)?)\s*M(?:illion)?(?:\s*(?:token|context|ctx|window))?/i.exec(
		normalized,
	)
	if (millionMatch) return Math.round(Number.parseFloat(millionMatch[1]) * 1_000_000)

	const thousandMatch = /(\d+(?:\.\d+)?)\s*K(?:\s*(?:token|context|ctx|window))?/i.exec(normalized)
	if (thousandMatch) return Math.round(Number.parseFloat(thousandMatch[1]) * 1_000)

	const explicitMatch = /(\d{4,})\s*(?:tokens?|context|ctx)/i.exec(normalized)
	if (explicitMatch) return Number.parseInt(explicitMatch[1], 10)

	return null
}

function inferKnownModelLimit(modelValue: string): number | null {
	const value = modelValue.toLowerCase()
	if (value.includes("minimax")) return 1_000_000
	if (value.includes("kimi-k2")) return 262_144
	if (value.includes("deepseek")) return 128_000
	if (value.includes("glm")) return 128_000
	if (value.includes("qwen")) return 128_000
	if (value.includes("grok")) return 131_072
	return null
}

/** Infer the active model's context limit from config metadata when ACP has not sent usage yet. */
export function inferModelContextLimit(configOptions: ConfigOption[]): number | null {
	const modelOption = findModelOption(configOptions)
	if (!modelOption) return null

	const selected = modelOption.options.find((entry) => entry.value === modelOption.currentValue)
	const texts = [
		selected?.description,
		selected?.name,
		selected?.value,
		modelOption.currentValue,
	].filter((entry): entry is string => Boolean(entry?.trim()))

	for (const text of texts) {
		const parsed = parseTokenLimitFromText(text)
		if (parsed) return parsed
	}

	return inferKnownModelLimit(modelOption.currentValue)
}

/** Merge live ACP usage with model metadata so new sessions still show total capacity. */
export function resolveContextWindowDisplay(
	usage: ContextWindowSnapshot | null,
	configOptions: ConfigOption[],
): ContextWindowSnapshot | null {
	const modelLimit = inferModelContextLimit(configOptions)

	if (!usage) {
		if (!modelLimit) return null
		return {
			usedTokens: 0,
			maxTokens: modelLimit,
			usedPercent: 0,
			costUsd: null,
			breakdown: [],
			updatedAt: Date.now(),
		}
	}

	const maxTokens = usage.maxTokens ?? modelLimit
	const usedPercent =
		usage.usedPercent ??
		(maxTokens && maxTokens > 0 ? clampPercent((usage.usedTokens / maxTokens) * 100) : null)

	return {
		...usage,
		maxTokens,
		usedPercent,
	}
}

export function hasContextWindowData(snapshot: ContextWindowSnapshot | null): boolean {
	return snapshot !== null && (snapshot.maxTokens !== null || snapshot.usedTokens > 0)
}

export function formatAvailableContextTokens(snapshot: ContextWindowSnapshot): string | null {
	if (!snapshot.maxTokens) return null
	const available = Math.max(0, snapshot.maxTokens - snapshot.usedTokens)
	return formatContextTokens(available)
}