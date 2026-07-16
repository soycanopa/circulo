import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import type {
	AgentCapabilities,
	ConfigOption,
	CredentialResponse,
	SessionInfo,
} from "@/types/acp"

export interface ProjectStatus {
	connected: boolean
	projectPath: string | null
	sessionId: string | null
	activeSessionId: string | null
	agentCommand: string
	sessions: SessionInfo[]
	capabilities: AgentCapabilities | null
}

export async function getProjectStatus(): Promise<ProjectStatus> {
	return invoke<ProjectStatus>("get_project_status")
}

export async function openProject(path: string): Promise<ProjectStatus> {
	return invoke<ProjectStatus>("open_project", { path })
}

export async function closeProject(): Promise<ProjectStatus> {
	return invoke<ProjectStatus>("close_project")
}

export async function sendPrompt(text: string, contextPaths: string[]): Promise<void> {
	return invoke("send_prompt", { text, contextPaths })
}

export async function respondPermission(requestId: string, optionId: string): Promise<void> {
	return invoke("respond_permission", { requestId, optionId })
}

export async function respondCredential(
	requestId: string,
	response: CredentialResponse,
): Promise<void> {
	return invoke("respond_credential", { requestId, response })
}

export async function setConfigOption(configId: string, value: string): Promise<void> {
	return invoke("set_config_option", { configId, value })
}

export async function searchFiles(query: string): Promise<string[]> {
	return invoke<string[]>("search_files", { query })
}

export interface OpencodeCommandEntry {
	name: string
	description: string | null
	scope: string
}

export interface OpencodeSkillEntry {
	name: string
	path: string
	description: string | null
	scope: string
}

export interface OpencodeMcpServerEntry {
	name: string
	enabled: boolean
	scope: string
	serverType: string | null
	source: string
	configPath: string
	readOnly: boolean
}

export async function listOpencodeCommands(
	projectPath: string | null,
): Promise<OpencodeCommandEntry[]> {
	return invoke<OpencodeCommandEntry[]>("list_opencode_commands", {
		projectPath,
	})
}

export async function listOpencodeSkills(
	projectPath: string | null,
): Promise<OpencodeSkillEntry[]> {
	return invoke<OpencodeSkillEntry[]>("list_opencode_skills", {
		projectPath,
	})
}

export async function listOpencodeMcpServers(
	projectPath: string | null,
): Promise<OpencodeMcpServerEntry[]> {
	return invoke<OpencodeMcpServerEntry[]>("list_opencode_mcp_servers", {
		projectPath,
	})
}

export async function setOpencodeMcpEnabled(input: {
	name: string
	scope: string
	enabled: boolean
	projectPath: string | null
	configPath: string | null
}): Promise<void> {
	return invoke("set_opencode_mcp_enabled", input)
}

export async function listSessions(): Promise<ProjectStatus> {
	return invoke<ProjectStatus>("list_sessions")
}

export async function createSession(): Promise<ProjectStatus> {
	return invoke<ProjectStatus>("create_session")
}

export async function loadSession(id: string): Promise<ProjectStatus> {
	return invoke<ProjectStatus>("load_session", { id })
}

export async function closeSession(id: string): Promise<ProjectStatus> {
	return invoke<ProjectStatus>("close_session", { id })
}

export async function renameSession(id: string, title: string): Promise<ProjectStatus> {
	return invoke<ProjectStatus>("rename_session", { id, title })
}

export function listenAcpEvents(handlers: {
	onSessionReady?: (payload: {
		sessionId: string
		projectPath: string
		configOptions: ConfigOption[]
	}) => void
	onSessionsUpdated?: (payload: {
		sessions: SessionInfo[]
		activeSessionId: string
	}) => void
	onSessionUpdate?: (payload: unknown) => void
	onPermissionRequest?: (payload: unknown) => void
	onCredentialRequest?: (payload: unknown) => void
	onConfigOptions?: (payload: { configOptions: ConfigOption[] }) => void
	onPromptComplete?: () => void
	onError?: (payload: { message: string }) => void
	onDisconnected?: () => void
}): Promise<UnlistenFn[]> {
	return Promise.all([
		listen("acp:session_ready", (event) => {
			handlers.onSessionReady?.(event.payload as {
				sessionId: string
				projectPath: string
				configOptions: ConfigOption[]
			})
		}),
		listen("acp:sessions_updated", (event) => {
			handlers.onSessionsUpdated?.(event.payload as {
				sessions: SessionInfo[]
				activeSessionId: string
			})
		}),
		listen("acp:session_update", (event) => {
			handlers.onSessionUpdate?.(event.payload)
		}),
		listen("acp:permission_request", (event) => {
			handlers.onPermissionRequest?.(event.payload)
		}),
		listen("acp:credential_request", (event) => {
			handlers.onCredentialRequest?.(event.payload)
		}),
		listen("acp:config_options", (event) => {
			handlers.onConfigOptions?.(event.payload as { configOptions: ConfigOption[] })
		}),
		listen("acp:prompt_complete", () => {
			handlers.onPromptComplete?.()
		}),
		listen("acp:error", (event) => {
			handlers.onError?.(event.payload as { message: string })
		}),
		listen("agent:disconnected", () => {
			handlers.onDisconnected?.()
		}),
	])
}
