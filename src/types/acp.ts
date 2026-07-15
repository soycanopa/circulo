export type SessionStatus = "idle" | "generating" | "awaiting_permission" | "disconnected"

export interface ConfigOption {
	id: string
	name: string
	category?: string
	currentValue: string
	options: Array<{
		value: string
		name: string
		description?: string
		group?: string
	}>
}

export interface AgentCapabilities {
	loadSession: boolean
	listSessions: boolean
	resumeSession: boolean
	closeSession: boolean
}

export interface SessionInfo {
	sessionId: string
	cwd: string
	additionalDirectories: string[]
	title?: string
	updatedAt?: string
}

export interface PermissionOption {
	optionId: string
	name: string
	kind: "allow_once" | "allow_always" | "reject_once" | "reject_always" | string
}

export interface PermissionRequest {
	requestId: string
	sessionId: string
	toolCall: Record<string, unknown>
	options: PermissionOption[]
}

export type ToolCallStatus = "pending" | "in_progress" | "completed" | "failed"

export interface ToolCallState {
	id: string
	title: string
	kind?: string
	status: ToolCallStatus
	content: string
	diff?: {
		path: string
		oldText?: string
		newText: string
	}
	rawInput?: unknown
	rawOutput?: unknown
}

export interface ChatMessage {
	id: string
	role: "user" | "assistant"
	content: string
	toolCalls: ToolCallState[]
	timestamp: number
}

export interface MentionChip {
	path: string
	label: string
}

export type SidebarSessionStatus = "running" | "waiting" | "idle" | "failed"
