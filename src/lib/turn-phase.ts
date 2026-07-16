import type { SessionStatus, ToolCallState } from "@/types/acp"

export type TurnPhase = "idle" | "pending" | "tool_active" | "awaiting" | "streaming" | "complete"

export interface DeriveTurnPhaseInput {
	promptInFlight: boolean
	sessionStatus: SessionStatus
	streamingText: string
	toolCalls: ToolCallState[]
}

function hasActiveTool(toolCalls: ToolCallState[]): boolean {
	return toolCalls.some(
		(tool) => tool.status === "pending" || tool.status === "in_progress",
	)
}

/** Lightweight turn phase for the active prompt (Craft-style, ACP-adapted). */
export function deriveTurnPhase({
	promptInFlight,
	sessionStatus,
	streamingText,
	toolCalls,
}: DeriveTurnPhaseInput): TurnPhase {
	if (!promptInFlight) return "idle"
	if (sessionStatus === "awaiting_permission" || sessionStatus === "awaiting_credential") {
		return "awaiting"
	}
	if (hasActiveTool(toolCalls)) return "tool_active"
	if (streamingText.trim()) return "streaming"
	return "pending"
}

export function shouldShowThinkingIndicator(phase: TurnPhase): boolean {
	return phase === "pending" || phase === "tool_active"
}