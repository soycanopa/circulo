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
	/** Multiple ACP sessions may run concurrently against the same agent process. */
	concurrentSessions: boolean
}

export interface ProjectStatus {
	connected: boolean
	projectPath: string | null
	agentId: string | null
	connectionGeneration: number | null
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
	connectionGeneration?: number
	toolCall?: unknown
	options: PermissionOption[]
}

export interface ToolCallDiff {
	type: "diff"
	path: string
	oldText: string
	newText: string
}

export interface ToolCallTerminal {
	type: "terminal"
	terminalId: string
}

export type ToolCallContent = string | ToolCallDiff | ToolCallTerminal

export interface ToolCall {
	id: string
	title: string
	kind?: string
	status: string
	content?: ToolCallContent
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
	createdAt?: number
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

export interface TerminalState {
	terminalId: string
	sessionId: string
	label: string
	output: string
	truncated: boolean
	running: boolean
	exitStatus: { exitCode?: number; signal?: string } | null
}

export interface RecentProject {
	path: string
	lastOpenedAt: number
}

export interface WorkspaceEntry {
	id: string
	/** Project folders that belong only to this space. */
	projectPaths: string[]
	/** Last cwd opened in this space. */
	lastPath?: string | null
	createdAt: number
}

export interface AppSettings {
	version: number
	recentProjects: RecentProject[]
	workspaces: WorkspaceEntry[]
	activeWorkspaceId: string | null
	preferredAgentId?: string | null
	enabledAgentIds?: string[]
	favoriteModelIds?: string[]
	autoApproveEnabled?: boolean
}

export interface AgentDescriptor {
	id: string
	label: string
	command: string
	available: boolean
}

export interface Automation {
	id: string
	title: string
	prompt: string
	createdAt: number
	updatedAt: number
}
