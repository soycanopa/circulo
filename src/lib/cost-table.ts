/**
 * Estimated per-token costs for common models, USD per 1M tokens.
 * Used only to display rough cost estimates in Settings > Usage — prices change,
 * so the dashboard labels every figure as "estimated".
 */
export const COST_TABLE: Record<string, { input: number; output: number }> = {
	// Anthropic Claude
	"claude-sonnet-4-5": { input: 3.0, output: 15.0 },
	"claude-opus-4-5": { input: 5.0, output: 25.0 },
	"claude-sonnet-4": { input: 3.0, output: 15.0 },
	"claude-opus-4": { input: 15.0, output: 75.0 },
	"claude-3-7-sonnet": { input: 3.0, output: 15.0 },
	"claude-3-5-sonnet": { input: 3.0, output: 15.0 },
	"claude-3-5-haiku": { input: 0.8, output: 4.0 },
	// OpenAI GPT
	"gpt-4o": { input: 2.5, output: 10.0 },
	"gpt-4o-mini": { input: 0.15, output: 0.6 },
	"gpt-5": { input: 1.25, output: 10.0 },
	// Google Gemini
	"gemini-2.5-pro": { input: 1.25, output: 10.0 },
	"gemini-2.5-flash": { input: 0.3, output: 2.5 },
	// Meta Llama (typical provider pricing)
	"llama-4": { input: 0.25, output: 0.25 },
	"llama-3.3": { input: 0.25, output: 0.25 },
}

const DEFAULT_COST = { input: 1.0, output: 3.0 }

/**
 * Estimate the cost of a usage sample. We only know total tokens used per
 * sample, so we approximate input/output split 70/30. Output is marked
 * "estimated" in the UI.
 */
export function estimateSampleCost(model: string | undefined, tokens: number): number {
	const rates = COST_TABLE[model ?? ""] ?? DEFAULT_COST
	const inputTokens = tokens * 0.7
	const outputTokens = tokens * 0.3
	return (inputTokens * rates.input + outputTokens * rates.output) / 1_000_000
}

export function formatUsd(cents: number): string {
	if (cents < 1) return `${(cents * 100).toFixed(2)}¢`
	return `$${cents.toFixed(2)}`
}
