export interface ConfigOptionValue {
	value: string
	name: string
	description?: string | null
	group?: string | null
}

export interface ConfigOption {
	id: string
	name: string
	category?: string | null
	currentValue: string
	options: ConfigOptionValue[]
}

export interface AgentCapabilities {
	loadSession: boolean
	listSessions: boolean
	resumeSession: boolean
	closeSession: boolean
}

export interface ProjectStatus {
	connected: boolean
	projectPath: string | null
	agentId: string | null
	sessionId: string | null
	configOptions: ConfigOption[]
	capabilities: AgentCapabilities | null
	agentCommand: string
}

export interface PermissionOption {
	optionId: string
	name: string
	kind?: string
}

export interface PermissionRequest {
	requestId: string
	sessionId: string
	toolCall?: unknown
	options: PermissionOption[]
}

export interface ToolCall {
	id: string
	title: string
	kind?: string
	status: string
	content?: string
	rawInput?: unknown
	rawOutput?: unknown
}

export interface ChatMessage {
	id: string
	role: "user" | "assistant"
	content: string
	toolCalls: ToolCall[]
	timestamp: number
}

export type SessionStatus =
	| "idle"
	| "connecting"
	| "generating"
	| "awaiting_permission"
	| "disconnected"

export interface OpencodeStatus {
	available: boolean
	path: string | null
	installHint: string
}

export interface ChatSessionSummary {
	sessionId: string
	title: string
	updatedAt: number
}

export interface StoredChatMessage {
	id: string
	role: "user" | "assistant"
	content: string
	toolCalls: ToolCall[]
	timestamp: number
}

export interface StoredTranscript {
	sessionId: string
	projectPath: string
	title: string
	createdAt: number
	updatedAt: number
	messages: StoredChatMessage[]
}

export interface RecentProject {
	path: string
	lastOpenedAt: number
}

export interface AppSettings {
	version: number
	recentProjects: RecentProject[]
}
