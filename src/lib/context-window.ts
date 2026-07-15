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

/** Parse OpenCode ACP `usage_update` (and optional extended breakdown fields). */
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