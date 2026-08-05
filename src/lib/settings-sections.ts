import {
	Blocks,
	Bot,
	Command,
	Info,
	LayoutGrid,
	Plug,
	ShieldCheck,
	SlidersHorizontal,
	Sparkles,
	Zap,
	type LucideIcon,
} from "lucide-react"

export type SettingsSectionId =
	| "general"
	| "agents"
	| "models"
	| "automations"
	| "slash"
	| "permissions"
	| "workspaces"
	| "mcp"
	| "skills"
	| "usage"
	| "about"

export interface SettingsSectionDef {
	id: SettingsSectionId
	label: string
	description: string
	icon: LucideIcon
}

export const SETTINGS_SECTIONS: SettingsSectionDef[] = [
	{
		id: "general",
		label: "General",
		description: "Chats folder, data locations",
		icon: SlidersHorizontal,
	},
	{
		id: "agents",
		label: "Agents",
		description: "Enable agents, default agent, ACP command",
		icon: Bot,
	},
	{
		id: "models",
		label: "Models",
		description: "Favorite and recent models",
		icon: Sparkles,
	},
	{
		id: "automations",
		label: "Automations",
		description: "Saved prompts for the command palette",
		icon: Zap,
	},
	{
		id: "slash",
		label: "Slash commands",
		description: "Custom composer commands",
		icon: Command,
	},
	{
		id: "permissions",
		label: "Permissions",
		description: "Auto-edit and always-allow tools",
		icon: ShieldCheck,
	},
	{
		id: "workspaces",
		label: "Workspaces",
		description: "Spaces and recent projects",
		icon: LayoutGrid,
	},
	{
		id: "mcp",
		label: "MCP servers",
		description: "Orchestrator, presets, imports and auto-load",
		icon: Plug,
	},
	{
		id: "skills",
		label: "Skills",
		description: "Install skills from skills.sh to projects or globally",
		icon: Blocks,
	},
	{
		id: "usage",
		label: "Usage",
		description: "Token tracking and measured savings",
		icon: Zap,
	},
	{
		id: "about",
		label: "About",
		description: "Version and data locations",
		icon: Info,
	},
]
