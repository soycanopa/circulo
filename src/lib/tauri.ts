import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import type {
	AgentCapabilities,
	AppSettings,
	ChatMessage,
	ChatSessionSummary,
	ConfigOption,
	PermissionRequest,
	ProjectStatus,
	StoredChatMessage,
	StoredTranscript,
} from "@/types/acp"

export async function getProjectStatus(): Promise<ProjectStatus> {
	return invoke("get_project_status")
}

export async function getDefaultChatsPath(): Promise<string> {
	return invoke("get_default_chats_path")
}

export interface OpencodeStatus {
	available: boolean
	path: string | null
	installHint: string
}

export async function checkOpencode(): Promise<OpencodeStatus> {
	return invoke("check_opencode")
}

export async function getHomePath(): Promise<string> {
	return invoke("get_home_path")
}

export async function openProject(
	path: string,
	agentId?: string,
): Promise<ProjectStatus> {
	return invoke("open_project", { path, agentId: agentId ?? null })
}

export async function closeProject(): Promise<ProjectStatus> {
	return invoke("close_project")
}

export async function createSession(): Promise<ProjectStatus> {
	return invoke("create_session")
}

export async function loadSession(sessionId: string): Promise<ProjectStatus> {
	return invoke("load_session", { sessionId })
}

export async function closeSession(sessionId: string): Promise<ProjectStatus> {
	return invoke("close_session_cmd", { sessionId })
}

export async function sendPrompt(
	text: string,
	contextPaths: string[],
): Promise<void> {
	return invoke("send_prompt", { text, contextPaths })
}

export async function cancelPrompt(): Promise<void> {
	return invoke("cancel_prompt")
}

export async function respondPermission(
	requestId: string,
	optionId: string,
): Promise<void> {
	return invoke("respond_permission", { requestId, optionId })
}

export async function setConfigOption(
	configId: string,
	value: string,
): Promise<void> {
	return invoke("set_config_option", { configId, value })
}

export async function searchFiles(query: string): Promise<string[]> {
	return invoke("search_files", { query })
}

export async function pickDirectory(): Promise<string | null> {
	return invoke("pick_directory")
}

export async function getAppSettings(): Promise<AppSettings> {
	return invoke("get_app_settings")
}

export async function listChatSessions(
	projectPath: string,
): Promise<ChatSessionSummary[]> {
	return invoke("list_chat_sessions_cmd", { projectPath })
}

export async function loadChatTranscript(
	projectPath: string,
	sessionId: string,
): Promise<StoredTranscript> {
	return invoke("load_chat_transcript_cmd", { projectPath, sessionId })
}

export async function saveChatTranscript(
	projectPath: string,
	sessionId: string,
	messages: StoredChatMessage[],
): Promise<ChatSessionSummary> {
	return invoke("save_chat_transcript_cmd", {
		projectPath,
		sessionId,
		messages,
	})
}

export async function deleteChatTranscript(
	projectPath: string,
	sessionId: string,
): Promise<void> {
	return invoke("delete_chat_transcript_cmd", { projectPath, sessionId })
}

export async function renameChatTranscript(
	projectPath: string,
	sessionId: string,
	title: string,
): Promise<ChatSessionSummary> {
	return invoke("rename_chat_transcript_cmd", { projectPath, sessionId, title })
}

export async function exportTranscript(
	filename: string,
	content: string,
): Promise<boolean> {
	return invoke("export_transcript_cmd", { filename, content })
}

function toStoredMessages(messages: ChatMessage[]): StoredChatMessage[] {
	return messages.map((m) => ({
		id: m.id,
		role: m.role,
		content: m.content,
		toolCalls: m.toolCalls,
		timestamp: m.timestamp,
	}))
}

export async function persistChatTranscript(
	projectPath: string,
	sessionId: string,
	messages: ChatMessage[],
): Promise<ChatSessionSummary> {
	return saveChatTranscript(
		projectPath,
		sessionId,
		toStoredMessages(messages),
	)
}

export function listenAcpEvents(handlers: {
	onAgentReady?: (payload: {
		projectPath: string
		capabilities: AgentCapabilities
	}) => void
	onSessionReady?: (payload: {
		sessionId: string
		projectPath: string
		configOptions: ConfigOption[]
		resume?: boolean
	}) => void
	onSessionUpdate?: (payload: unknown) => void
	onPermissionRequest?: (payload: PermissionRequest) => void
	onConfigOptions?: (payload: {
		configOptions: ConfigOption[]
		sessionId?: string
	}) => void
	onPromptComplete?: (payload?: { sessionId?: string }) => void
	onError?: (payload: { message: string; sessionId?: string }) => void
	onDisconnected?: () => void
	onProgress?: (payload: { phase: string; message?: string }) => void
}): Promise<UnlistenFn[]> {
	return Promise.all([
		listen("agent:ready", (event) => {
			handlers.onAgentReady?.(
				event.payload as {
					projectPath: string
					capabilities: AgentCapabilities
				},
			)
		}),
		listen("acp:session_ready", (event) => {
			handlers.onSessionReady?.(
				event.payload as {
					sessionId: string
					projectPath: string
					configOptions: ConfigOption[]
					resume?: boolean
				},
			)
		}),
		listen("acp:session_update", (event) => {
			handlers.onSessionUpdate?.(event.payload)
		}),
		listen("acp:permission_request", (event) => {
			handlers.onPermissionRequest?.(event.payload as PermissionRequest)
		}),
		listen("acp:config_options", (event) => {
			handlers.onConfigOptions?.(
				event.payload as { configOptions: ConfigOption[]; sessionId?: string },
			)
		}),
		listen("acp:prompt_complete", (event) => {
			handlers.onPromptComplete?.(event.payload as { sessionId?: string })
		}),
		listen("acp:error", (event) => {
			handlers.onError?.(event.payload as { message: string; sessionId?: string })
		}),
		listen("agent:disconnected", () => {
			handlers.onDisconnected?.()
		}),
		listen("agent:progress", (event) => {
			handlers.onProgress?.(event.payload as { phase: string; message?: string })
		}),
	])
}
