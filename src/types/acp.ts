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
	/** MCP transport support advertised in `initialize`. Stdio is mandatory per ACP. */
	mcpStdio: boolean
	mcpHttp: boolean
	mcpSse: boolean
	/**
	 * Empirically observed: the agent delegated a `terminal/*` request to the
	 * client. Grok does; OpenCode runs bash internally.
	 */
	terminalDelegation: boolean
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

export interface GitFileStatus {
	path: string
	status: "created" | "modified" | "deleted" | "untracked"
	staged: boolean
}

export interface GitStatus {
	branch: string
	files: GitFileStatus[]
}

export interface GitBranchInfo {
	name: string
	current: boolean
	remote: boolean
	upstream: string | null
}

export interface GitBranches {
	current: string
	detached: boolean
	local: GitBranchInfo[]
	remote: GitBranchInfo[]
}

export interface ToolCall {
	id: string
	title: string
	kind?: string
	status: string
	content?: ToolCallContent
	rawInput?: unknown
	rawOutput?: unknown
	/** Set when the tool call delegates to a sub-agent (e.g. OpenCode `task`). */
	taskState?: "pending" | "running" | "completed" | "failed"
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
	recentModelIds?: string[]
	autoApproveEnabled?: boolean
	/** Tool patterns the user chose to always allow (exact or simple glob). */
	allowedToolPatterns?: string[]
	/** User-defined slash commands shown in the composer menu. */
	customSlashCommands?: CustomSlashCommand[]
	/** Optional Vercel OIDC token for the authenticated skills.sh /api/v1 API. */
	vercelOidcToken?: string | null
}

export interface CustomSlashCommand {
	command: string
	label: string
	description: string
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

// ---------------------------------------------------------------------------
// MCP registry (Settings > MCP)
// ---------------------------------------------------------------------------

export type McpServerKind = "stdio" | "http" | "sse"

export interface McpEnvVar {
	name: string
	value: string
}

export interface ManagedMcpServer {
	id: string
	name: string
	kind: McpServerKind
	/** Stdio: executable path. Http/Sse: server URL. */
	command: string
	args: string[]
	env: McpEnvVar[]
	/** Eligible for on-demand loading via `/mcp name` → `mcp_load`. */
	enabled: boolean
	/** Injected natively into `session/new` with the full tool catalogue. */
	autoLoad: boolean
	/** Built-in servers (e.g. the orchestrator) cannot be deleted. */
	builtIn: boolean
}

export interface McpImportCandidate {
	id: string
	name: string
	kind: McpServerKind
	command: string
	args: string[]
	env: McpEnvVar[]
	/** Config file the server came from (`.mcp.json` / `opencode.json`). */
	source: string
}

export interface McpValidationResult {
	ok: boolean
	error: string | null
	tools: string[]
	toolCount: number
}

export interface CirculoMcpStatus {
	available: boolean
	path: string | null
	registryPath: string
}

// ---------------------------------------------------------------------------
// Skills (Settings > Skills)
// ---------------------------------------------------------------------------

export interface SkillSearchResult {
	/** Stable `{source}/{slug}` id (skills.sh). */
	id: string
	/** Legacy search endpoint's `skillId` (also the v1 `slug`). */
	skillId: string
	slug: string
	name: string
	installs: number
	source: string
	description: string
	/** GitHub repo URL / well-known base (authenticated API only). */
	installUrl: string | null
	/** Link to the skill page on skills.sh (authenticated API only). */
	url: string | null
	/** "github" | "well-known" (authenticated API only). */
	sourceType: string | null
}

export interface SkillSearchResponse {
	skills: SkillSearchResult[]
	count: number
	/** "authenticated" (official /api/v1) or "public" (fallback endpoint). */
	mode: string
	/** True when the chosen skills.sh path failed/changed. */
	degraded: boolean
	error: string | null
}

export interface InstalledSkill {
	name: string
	description: string
	/** "project" | "global" */
	scope: string
	path: string
}

export interface SkillListResponse {
	skills: InstalledSkill[]
}
