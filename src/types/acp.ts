export type SessionStatus =
	| "idle"
	| "generating"
	| "awaiting_permission"
	| "awaiting_credential"
	| "disconnected"

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

export type CredentialAuthMode = "bearer" | "basic" | "header" | "multi-header" | "url"

export interface CredentialField {
	key: string
	label: string
	placeholder?: string
	secret?: boolean
	required?: boolean
}

export interface CredentialRequest {
	requestId: string
	sessionId: string
	toolCallId?: string
	title: string
	description?: string
	mode: CredentialAuthMode
	fields: CredentialField[]
	sourceUrl?: string
	url?: string
	serviceName?: string
}

export type CredentialResponseAction = "accept" | "decline" | "cancel"

export interface CredentialResponse {
	action: CredentialResponseAction
	values?: Record<string, string>
}

export type ChatMessageKind = "chat" | "auth-request"

export interface ChatMessage {
	id: string
	role: "user" | "assistant"
	content: string
	toolCalls: ToolCallState[]
	timestamp: number
	kind?: ChatMessageKind
	authMeta?: {
		title: string
		mode: CredentialAuthMode
		status: "provided" | "declined" | "cancelled"
	}
}

export interface MentionChip {
	path: string
	label: string
}

export type SidebarSessionStatus = "running" | "waiting" | "idle" | "failed"
