import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import type { ConfigOption } from "@/types/acp"

export interface ProjectStatus {
	connected: boolean
	projectPath: string | null
	sessionId: string | null
	agentCommand: string
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

export async function setConfigOption(configId: string, value: string): Promise<void> {
	return invoke("set_config_option", { configId, value })
}

export async function searchFiles(query: string): Promise<string[]> {
	return invoke<string[]>("search_files", { query })
}

export function listenAcpEvents(handlers: {
	onSessionReady?: (payload: {
		sessionId: string
		projectPath: string
		configOptions: ConfigOption[]
	}) => void
	onSessionUpdate?: (payload: unknown) => void
	onPermissionRequest?: (payload: unknown) => void
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
		listen("acp:session_update", (event) => {
			handlers.onSessionUpdate?.(event.payload)
		}),
		listen("acp:permission_request", (event) => {
			handlers.onPermissionRequest?.(event.payload)
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