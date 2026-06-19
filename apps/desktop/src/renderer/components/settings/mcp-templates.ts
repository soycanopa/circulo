import type { McpLocalConfig, McpRemoteConfig } from "@opencode-ai/sdk/v2/client"

// ============================================================
// Types
// ============================================================

export type McpInstallType = "simple" | "agent"

export interface McpTemplate {
	id: string
	name: string
	description: string
	/** lucide-react icon name, resolved in the card component */
	icon: string
	type: McpInstallType
	/** For simple type: config passed to client.mcp.add() */
	config?: McpLocalConfig | McpRemoteConfig
	/** Environment variables the user needs to provide (simple type only) */
	envVars?: McpTemplateEnvVar[]
	/** For agent type: prompt sent to the background agent */
	prompt?: string
}

export interface McpTemplateEnvVar {
	key: string
	label: string
	helpUrl?: string
}

// ============================================================
// Helpers
// ============================================================

export function isTemplateInstalled(
	templateId: string,
	installedNames: string[],
): boolean {
	return installedNames.includes(templateId)
}

// ============================================================
// Template Definitions
// ============================================================

export const MCP_TEMPLATES: McpTemplate[] = [
	{
		id: "figma",
		name: "Figma MCP",
		description: "Connect to Figma designs and export assets directly",
		icon: "paintbrush",
		type: "simple",
		config: {
			type: "local",
			command: ["npx", "-y", "figma-developer-mcp", "--stdio"],
		},
		envVars: [
			{
				key: "FIGMA_PERSONAL_ACCESS_TOKEN",
				label: "Figma Personal Access Token",
				helpUrl: "https://www.figma.com/developers/api#access-tokens",
			},
		],
	},
	{
		id: "github",
		name: "GitHub MCP",
		description: "Manage repos, issues, PRs, and browse code",
		icon: "github",
		type: "simple",
		config: {
			type: "local",
			command: ["npx", "-y", "@anthropic-ai/mcp-server-github"],
		},
		envVars: [
			{
				key: "GITHUB_PERSONAL_ACCESS_TOKEN",
				label: "GitHub Personal Access Token",
				helpUrl: "https://github.com/settings/tokens",
			},
		],
	},
	{
		id: "filesystem",
		name: "Filesystem MCP",
		description: "Read, write, and manage files on your machine",
		icon: "folder-tree",
		type: "simple",
		config: {
			type: "local",
			command: [
				"npx",
				"-y",
				"@anthropic-ai/mcp-server-filesystem",
				"/Users",
			],
		},
	},
	{
		id: "framer",
		name: "Framer MCP",
		description: "Design and prototype interactive websites",
		icon: "layout",
		type: "agent",
		prompt:
			"Please install the Framer MCP server. Research the current installation method for Framer MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "wonder",
		name: "Wonder MCP",
		description: "AI-powered design tool integration",
		icon: "wand",
		type: "agent",
		prompt:
			"Please install the Wonder MCP server. Research the current installation method for Wonder MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "paper-design",
		name: "Paper Design MCP",
		description: "Design-to-code workflow automation",
		icon: "pencil-ruler",
		type: "agent",
		prompt:
			"Please install the Paper Design MCP server. Research the current installation method for Paper Design MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "supabase",
		name: "Supabase MCP",
		description: "Backend database and auth for your products",
		icon: "database",
		type: "agent",
		prompt:
			"Please install the Supabase MCP server from @supabase/mcp-server-supabase. Research the current installation method, install it, configure any required environment variables (SUPABASE_ACCESS_TOKEN, SUPABASE_PROJECT_REF), and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "vercel",
		name: "Vercel MCP",
		description: "Deploy your products with one command",
		icon: "triangle",
		type: "agent",
		prompt:
			"Please install the Vercel MCP server. Research the current installation method for Vercel's MCP integration, install it with the appropriate package manager, configure any required environment variables (VERCEL_TOKEN), and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "sketch",
		name: "Sketch MCP",
		description: "Design tool for macOS",
		icon: "pen-tool",
		type: "agent",
		prompt:
			"Please install the Sketch MCP server. Research the current installation method for Sketch MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "craft",
		name: "Craft MCP",
		description: "Document & note-taking platform",
		icon: "file-text",
		type: "agent",
		prompt:
			"Please install the Craft MCP server (Craft.do). Research the current installation method for Craft MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "notion",
		name: "Notion MCP",
		description: "Knowledge base & project management",
		icon: "book-open",
		type: "agent",
		prompt:
			"Please install the Notion MCP server from @notionhq/notion-mcp-server. Research the current installation method, install it, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "pixso",
		name: "Pixso MCP",
		description: "Collaborative design platform",
		icon: "users",
		type: "agent",
		prompt:
			"Please install the Pixso MCP server. Research the current installation method for Pixso MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "miro",
		name: "Miro MCP",
		description: "Collaborative whiteboard platform",
		icon: "grid-2x2",
		type: "agent",
		prompt:
			"Please install the Miro MCP server. Research the current installation method for Miro MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "webflow",
		name: "Webflow MCP",
		description: "Visual web design & CMS platform",
		icon: "globe",
		type: "agent",
		prompt:
			"Please install the Webflow MCP server. Research the current installation method for Webflow MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "zeplin",
		name: "Zeplin MCP",
		description: "Design handoff & collaboration",
		icon: "link-2",
		type: "agent",
		prompt:
			"Please install the Zeplin MCP server. Research the current installation method for Zeplin MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "penpot",
		name: "Penpot MCP",
		description: "Open-source design & prototyping",
		icon: "palette",
		type: "agent",
		prompt:
			"Please install the Penpot MCP server. Research the current installation method for Penpot MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "spline",
		name: "Spline MCP",
		description: "3D design & interactive experiences",
		icon: "box",
		type: "agent",
		prompt:
			"Please install the Spline MCP server. Research the current installation method for Spline MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "canva",
		name: "Canva MCP",
		description: "Online design & publishing platform",
		icon: "image",
		type: "agent",
		prompt:
			"Please install the Canva MCP server. Research the current installation method for Canva MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
	{
		id: "rive",
		name: "Rive MCP",
		description: "Interactive animation runtime",
		icon: "play",
		type: "agent",
		prompt:
			"Please install the Rive MCP server. Research the current installation method for Rive MCP, install it with the appropriate package manager, configure any required environment variables or authentication, and add it to OpenCode's MCP configuration so it shows up in `mcp.status()`. Make sure to verify the installation works.",
	},
]
