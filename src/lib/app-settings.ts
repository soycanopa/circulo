import { GENERAL_CHAT_PROJECT } from "@/lib/preferences"

export type SettingsSection =
	| "general"
	| "profile"
	| "shortcuts"
	| "skills"
	| "mcp"
	| "about"

export type DefaultAgentProvider = "opencode"

export interface AppSettings {
	defaultAgentMode: string
	defaultProvider: DefaultAgentProvider
	chatsProjectPath: string
	showChatsInSidebar: boolean
	showPinnedInSidebar: boolean
}

const SETTINGS_KEY = "circulo-app-settings"

const DEFAULT_SETTINGS: AppSettings = {
	defaultAgentMode: "plan",
	defaultProvider: "opencode",
	chatsProjectPath: GENERAL_CHAT_PROJECT,
	showChatsInSidebar: true,
	showPinnedInSidebar: true,
}

function readRaw(): Partial<AppSettings> {
	try {
		const raw = localStorage.getItem(SETTINGS_KEY)
		if (!raw) return {}
		const parsed: unknown = JSON.parse(raw)
		return parsed && typeof parsed === "object" && !Array.isArray(parsed)
			? (parsed as Partial<AppSettings>)
			: {}
	} catch {
		return {}
	}
}

export function getAppSettings(): AppSettings {
	const stored = readRaw()
	return {
		...DEFAULT_SETTINGS,
		...stored,
		chatsProjectPath: stored.chatsProjectPath?.trim() || DEFAULT_SETTINGS.chatsProjectPath,
	}
}

export function persistAppSettings(patch: Partial<AppSettings>): AppSettings {
	const next = { ...getAppSettings(), ...patch }
	localStorage.setItem(SETTINGS_KEY, JSON.stringify(next))
	if (typeof window !== "undefined") {
		window.dispatchEvent(new CustomEvent("circulo:settings-changed", { detail: next }))
	}
	return next
}

export function getChatsProjectPath(): string {
	return getAppSettings().chatsProjectPath
}

export function getDefaultAgentMode(): string {
	return getAppSettings().defaultAgentMode
}

export function getShowChatsInSidebar(): boolean {
	return getAppSettings().showChatsInSidebar
}

export function getShowPinnedInSidebar(): boolean {
	return getAppSettings().showPinnedInSidebar
}

export const SETTINGS_SECTIONS: {
	id: SettingsSection
	label: string
	description: string
}[] = [
	{
		id: "general",
		label: "General",
		description: "Defaults, sidebar layout, and chats folder.",
	},
	{
		id: "profile",
		label: "Profile",
		description: "Context usage and session preferences.",
	},
	{
		id: "shortcuts",
		label: "Shortcuts",
		description: "Keyboard shortcuts for Circulo.",
	},
	{
		id: "skills",
		label: "Skills",
		description: "OpenCode skills available to the agent.",
	},
	{
		id: "mcp",
		label: "MCP",
		description: "Model Context Protocol server toggles.",
	},
	{
		id: "about",
		label: "About",
		description: "Version, license, and runtime details.",
	},
]

export function getSettingsSectionMeta(section: SettingsSection) {
	return SETTINGS_SECTIONS.find((entry) => entry.id === section) ?? SETTINGS_SECTIONS[0]!
}