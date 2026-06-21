import type { McpLocalConfig, McpRemoteConfig } from "@opencode-ai/sdk/v2/client"

// ============================================================
// Types
// ============================================================

export type McpInstallType = "simple" | "agent" | "remote" | "local-http"

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
	/** Installation documentation (for simple type) */
	documentation?: string
	/** For remote type: server URL */
	serverUrl?: string
	/** For local-http type: local server URL */
	localUrl?: string
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
		type: "remote",
		serverUrl: "https://mcp.figma.com/mcp",
		documentation:
			"Install Figma MCP to connect Figma designs to AI assistants.\n\n" +
			"This is the official Figma MCP server that brings Figma directly into your workflow.\n\n" +
			"**Installation:**\n" +
			"Add MCP connector in your AI client:\n" +
			"URL: https://mcp.figma.com/mcp\n\n" +
			"**Configuration (Claude in Terminal):**\n" +
			"Run: `claude mcp add --transport http figma https://mcp.figma.com/mcp`\n\n" +
			"**Requirements:**\n" +
			"- Figma account (any plan)\n" +
			"- OAuth authentication\n" +
			"- MCP client that supports Streamable HTTP\n\n" +
			"**Features:**\n" +
			"- Write to the canvas (remote server only)\n" +
			"- Generate code from selected frames\n" +
			"- Extract design context (variables, components, layout)\n" +
			"- Code smarter with Code Connect\n" +
			"- Generate Figma designs from web pages\n\n" +
			"**Note:** This is the official Figma MCP server. Rate limits apply to read operations (up to 6 tool calls/month for Starter plan).",
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
		name: "FramerLink MCP for Figma",
		description: "Connect to Figma designs and export assets directly",
		icon: "paintbrush",
		type: "simple",
		config: {
			type: "local",
			command: ["npx", "-y", "figma-developer-mcp"],
		},
		envVars: [
			{
				key: "FIGMA_API_KEY",
				label: "Figma API Key",
				helpUrl: "https://help.figma.com/hc/en-us/articles/8085703771159-Manage-personal-access-tokens",
			},
		],
		documentation:
			"Install Framelink MCP to connect your Figma files to AI coding agents.\n\n" +
			"This MCP server provides Figma layout and styling information to AI tools like Cursor.\n\n" +
			"**Installation:**\n" +
			"Run: `npx -y figma-developer-mcp`\n\n" +
			"**Configuration:**\n" +
			"Add to your MCP config:\n" +
			"```\n" +
			"{\n" +
			"  \"mcpServers\": {\n" +
			"    \"Framelink MCP for Figma\": {\n" +
			"      \"command\": \"npx\",\n" +
			"      \"args\": [\"-y\", \"figma-developer-mcp\", \"--figma-api-key=YOUR-KEY\", \"--stdio\"]\n" +
			"    }\n" +
			"  }\n" +
			"```\n\n" +
			"**Required Environment Variable:**\n" +
			"- `FIGMA_API_KEY`: Your Figma personal access token\n\n" +
			"**Setup:**\n" +
			"1. Go to Figma → Account Settings → Personal Access Tokens\n" +
			"2. Create a new token\n" +
			"3. Set the API key in your MCP configuration\n\n" +
			"**Features:**\n" +
			"- Figma layout and styling information\n" +
			"- Works with Cursor, Claude, and other AI coding tools\n" +
			"- Simplifies Figma API responses for AI\n" +
			"- One-shot implementation from Figma designs\n\n" +
			"**Note:** This is the official Framelink MCP server (figma-developer-mcp).",
	},
	{
		id: "wonder",
		name: "Wonder MCP",
		description: "AI-powered design tool integration",
		icon: "wand",
		type: "remote",
		serverUrl: "https://mcp.wonder.so/mcp",
		documentation:
			"Install Wonder MCP to connect your Wonder designs to AI assistants.\n\n" +
			"Wonder MCP is Wonder's official MCP server.\n\n" +
			"**Installation:**\n" +
			"Add MCP connector in your AI client:\n" +
			"URL: https://mcp.wonder.so/mcp\n\n" +
			"**Authentication:**\n" +
			"- Requires Wonder account with Developer mode enabled\n" +
			"- OAuth authentication via browser\n\n" +
			"**Requirements:**\n" +
			"- Wonder account (wonder.design)\n" +
			"- Developer mode in Wonder settings\n" +
			"- MCP client that supports HTTP remote servers\n\n" +
			"**Features:**\n" +
			"- Read and query Wonder documents\n" +
			"- Search through your notes\n" +
			"- Update document content\n" +
			"- Manage blocks and attachments\n\n" +
			"**Note:** Developer mode must be enabled in Craft settings to use MCP.",
	},
	{
		id: "paper-design",
		name: "Paper Design MCP",
		description: "Design-to-code workflow automation",
		icon: "pencil-ruler",
		type: "simple",
		config: {
			type: "local",
			command: ["npx", "-y", "paper-design-mcp"],
		},
		envVars: [
			{
				key: "PAPER_COOKIE_STRING",
				label: "Paper Cookie String",
				helpUrl: "https://lobehub.com/mcp/garaevruslan-paper-design-mcp",
			},
		],
		documentation:
			"Install Paper Design MCP to connect Paper (paper.design) to AI assistants.\n\n**Installation:**\n`npx -y paper-design-mcp`\n\n**Required Environment Variable:**\n- `PAPER_COOKIE_STRING`: Full cookie string from Paper website\n\n**Setup:**\n1. Open Paper website in your browser\n2. Open Developer Tools → Application → Cookies\n3. Copy all cookies for paper.design\n4. Set the cookie string in your MCP configuration\n\n**Usage:**\nOnce installed, the MCP server connects to Paper's WebSocket API for canvas node operations.\n\n**Features:**\n- Canvas node operations\n- GPU shader integration\n- Design-to-code workflow automation\n- Vibe coding support\n\n**Requirements:**\n- Paper account (paper.design)\n- Full cookie string for authentication\n- Node.js environment\n\n**Note:** This is a community-built MCP server. Paper's official documentation provides more details about MCP integration.",
	},
	{
		id: "supabase",
		name: "Supabase MCP",
		description: "Backend database and auth for your products",
		icon: "database",
		type: "simple",
		config: {
			type: "local",
			command: ["npx", "-y", "@supabase/mcp-server-supabase"],
		},
		envVars: [
			{
				key: "SUPABASE_ACCESS_TOKEN",
				label: "Supabase Access Token",
				helpUrl: "https://supabase.com/docs/guides/ai-tools/mcp",
			},
			{
				key: "SUPABASE_PROJECT_REF",
				label: "Supabase Project Reference",
				helpUrl: "https://supabase.com/docs/guides/ai-tools/mcp",
			},
		],
		documentation:
			"Install Supabase MCP to connect your Supabase projects to AI assistants.\n\n**Installation:**\n`npx -y @supabase/mcp-server-supabase`\n\n**Required Environment Variables:**\n- `SUPABASE_ACCESS_TOKEN`: Your Supabase access token\n- `SUPABASE_PROJECT_REF`: Your Supabase project reference (e.g., `your-project-ref`)\n\n**Setup:**\n1. Go to Supabase Dashboard → Settings → API\n2. Copy your access token and project reference\n3. Set these environment variables in your MCP configuration\n\n**Usage:**\nOnce installed, the MCP server will provide tools to interact with your Supabase database, tables, and authentication.",
	},
	{
		id: "vercel",
		name: "Vercel MCP",
		description: "Deploy your products with one command",
		icon: "triangle",
		type: "remote",
		serverUrl: "https://mcp.vercel.com",
		documentation:
			"Install Vercel MCP to connect your Vercel projects to AI assistants.\n\n" +
			"Vercel MCP is Vercel's official MCP server (Beta).\n\n" +
			"**Installation:**\n" +
			"Run: `npx add-mcp https://mcp.vercel.com`\n" +
			"Or manually configure your MCP client:\n" +
			"URL: https://mcp.vercel.com\n\n" +
			"**Authentication:**\n" +
			"- Requires Vercel account with OAuth\n" +
			"- Automatically handles token refresh\n\n" +
			"**Requirements:**\n" +
			"- Vercel account (any plan)\n" +
			"- OAuth authentication\n" +
			"- MCP client that supports HTTP remote servers\n\n" +
			"**Features:**\n" +
			"- Search and navigate Vercel documentation\n" +
			"- Manage projects and deployments\n" +
			"- Analyze deployment logs\n\n" +
			"**Note:** Vercel MCP is available in Beta on all plans.",
	},
	{
		id: "sketch",
		name: "Sketch MCP",
		description: "Design tool for macOS",
		icon: "pen-tool",
		type: "local-http",
		localUrl: "http://localhost:31126/mcp",
		documentation:
			"Install Sketch MCP to connect Sketch to AI assistants.\n\n" +
			"Sketch MCP is a local integration within Sketch Desktop.\n\n" +
			"**Installation:**\n" +
			"1. Install and open Sketch Desktop (version >= 2.2.0)\n" +
			"2. Create or open a design file\n" +
			"3. Enable Sketch MCP in the file menu (top-left)\n" +
			"4. MCP server will start at: http://localhost:31126/mcp\n\n" +
			"**Configuration:**\n" +
			"Add MCP connector in your AI client:\n" +
			"URL: http://localhost:31126/mcp\n\n" +
			"**Requirements:**\n" +
			"- Sketch Desktop >= 2.2.0\n" +
			"- Enable Sketch MCP in file menu\n" +
			"- Keep Sketch Desktop open\n" +
			"- MCP client that supports HTTP local servers\n\n" +
			"**Features:**\n" +
			"- Copy layer link and paste in chat\n" +
			"- Select container on canvas\n" +
			"- Generate code from Sketch designs\n\n" +
			"**Note:** Keep Sketch Desktop open and the design file tab active for stable connection.",
	},
	{
		id: "craft",
		name: "Craft MCP",
		description: "Document & note-taking platform",
		icon: "file-text",
		type: "remote",
		serverUrl: "https://mcp.craft.do/my/mcp",
		documentation:
			"Install Craft MCP to connect your Craft documents to AI assistants.\n\n" +
			"Craft MCP is Craft's official MCP server.\n\n" +
			"**Installation:**\n" +
			"Add MCP connector in your AI client:\n" +
			"URL: https://mcp.craft.do/my/mcp\n\n" +
			"**Authentication:**\n" +
			"- Requires Craft account with Developer mode enabled\n" +
			"- OAuth authentication via browser\n\n" +
			"**Requirements:**\n" +
			"- Craft account (craft.do)\n" +
			"- Developer mode in Craft settings\n" +
			"- MCP client that supports HTTP remote servers\n\n" +
			"**Features:**\n" +
			"- Read and query Craft documents\n" +
			"- Search through your notes\n" +
			"- Update document content\n" +
			"- Manage blocks and attachments\n\n" +
			"**Note:** Developer mode must be enabled in Craft settings to use MCP.",
	},
	{
		id: "notion",
		name: "Notion MCP",
		description: "Knowledge base & project management",
		icon: "book-open",
		type: "simple",
		config: {
			type: "local",
			command: ["npx", "-y", "@notionhq/notion-mcp-server"],
		},
		envVars: [
			{
				key: "NOTION_API_KEY",
				label: "Notion API Key",
				helpUrl: "https://developers.notion.com/docs/get-started-with-mcp",
			},
		],
		documentation:
			"Install Notion MCP to connect your Notion workspace to AI assistants.\n\n**Installation:**\n`npx -y @notionhq/notion-mcp-server`\n\n**Required Environment Variable:**\n- `NOTION_API_KEY`: Your Notion integration API key\n\n**Setup:**\n1. Go to Notion → Create Integration\n2. Copy the integration token\n3. Add your workspace to the integration\n4. Set the API key in your MCP configuration\n\n**Usage:**\nOnce installed, the MCP server provides tools to:\n- Read and search pages\n- Query database views\n- Create and update content\n- Manage blocks and attachments\n\nThe server automatically uses the latest Notion API version (2026-03-11).",
	},
	{
		id: "pixso",
		name: "Pixso MCP",
		description: "Collaborative design platform",
		icon: "users",
		type: "local-http",
		localUrl: "http://127.0.0.1:3667/mcp",
		documentation:
			"Install Pixso MCP to connect your Pixso designs to AI assistants.\n\n" +
			"Pixso MCP is a local integration within Pixso Desktop.\n\n" +
			"**Installation:**\n" +
			"1. Install and open Pixso Desktop (version >= 2.2.0)\n" +
			"2. Create or open a design file\n" +
			"3. Enable Pixso MCP in the file menu (top-left)\n" +
			"4. MCP server will start at: http://127.0.0.1:3667/mcp\n\n" +
			"**Configuration:**\n" +
			"Add MCP connector in your AI client:\n" +
			"URL: http://127.0.0.1:3667/mcp\n\n" +
			"**Requirements:**\n" +
			"- Pixso Desktop >= 2.2.0\n" +
			"- Enable Pixso MCP in file menu\n" +
			"- Keep Pixso Desktop open\n" +
			"- MCP client that supports HTTP local servers\n\n" +
			"**Features:**\n" +
			"- Copy layer link and paste in chat\n" +
			"- Select container on canvas\n" +
			"- Generate code from Pixso designs\n\n" +
			"**Note:** Keep Pixso Desktop open and the design file tab active for stable connection.",
	},
	{
		id: "miro",
		name: "Miro MCP",
		description: "Collaborative whiteboard platform",
		icon: "grid-2x2",
		type: "simple",
		config: {
			type: "local",
			command: ["npx", "-y", "@llmindset/mcp-miro"],
		},
		envVars: [
			{
				key: "MIRO_API_TOKEN",
				label: "Miro Personal Access Token",
				helpUrl: "https://miro.com/integrations/mcp/",
			},
		],
		documentation:
			"Install Miro MCP to connect your Miro whiteboard to AI assistants.\n\n**Installation:**\n`npx -y @llmindset/mcp-miro`\n\n**Required Environment Variable:**\n- `MIRO_API_TOKEN`: Your Miro personal access token\n\n**Setup:**\n1. Go to Miro → Account Settings → API Tokens\n2. Create a new token\n3. Set the token in your MCP configuration\n\n**Usage:**\nOnce installed, the MCP server provides tools to:\n- Get board contents and structure\n- Create sticky notes, shapes, and frames\n- Read and analyze board content\n- Bulk create elements on the board\n- Instruct AI about board coordinates\n\n**Features:**\n- Full board access\n- Element creation and manipulation\n- Content analysis and extraction\n- Coordinate-based operations\n\n**Note:** This is a community-built MCP server for Miro. For official Miro MCP integration, check Miro's MCP documentation.",
	},
	{
		id: "webflow",
		name: "Webflow MCP",
		description: "Visual web design & CMS platform",
		icon: "globe",
		type: "simple",
		config: {
			type: "local",
			command: ["npx", "-y", "webflow/mcp-server"],
		},
		envVars: [
			{
				key: "WEBFLOW_TOKEN",
				label: "Webflow Access Token",
				helpUrl: "https://developers.webflow.com/mcp/reference/getting-started",
			},
		],
		documentation:
			"Install Webflow MCP to connect your Webflow projects to AI assistants.\n\n**Installation:**\n`npx -y webflow/mcp-server`\n\n**Required Environment Variable:**\n- `WEBFLOW_TOKEN`: Your Webflow access token\n\n**Setup:**\n1. Go to Webflow Developer Dashboard → API\n2. Generate an access token\n3. Set the token in your MCP configuration\n4. Make sure Node.js 22.3.0 or later is installed\n\n**Usage:**\nOnce installed, the MCP server provides tools to:\n- Read and query Webflow sites and pages\n- Manage collections and items\n- Access Webflow Data API\n- Generate code from Webflow designs\n\n**Requirements:**\n- Node.js 22.3.0 or higher\n- Webflow access token with appropriate permissions",
	},
	{
		id: "zeplin",
		name: "Zeplin MCP",
		description: "Design handoff & collaboration",
		icon: "link-2",
		type: "simple",
		config: {
			type: "local",
			command: ["npx", "-y", "@zeplin/mcp-server"],
		},
		envVars: [
			{
				key: "ZEPLIN_ACCESS_TOKEN",
				label: "Zeplin Personal Access Token",
				helpUrl: "https://support.zeplin.io/en/articles/11559086-zeplin-mcp-server",
			},
		],
		documentation:
			"Install Zeplin MCP to connect your Zeplin designs to AI assistants.\n\n**Installation:**\n`npx -y @zeplin/mcp-server`\n\n**Required Environment Variable:**\n- `ZEPLIN_ACCESS_TOKEN`: Your Zeplin personal access token\n\n**Setup:**\n1. Go to Zeplin → Settings → Account → Access Tokens\n2. Create a new token\n3. Set the token in your MCP configuration\n\n**Usage:**\nOnce installed, the MCP server provides tools to:\n- Read Zeplin project and design specifications\n- Export assets and code snippets\n- Access component libraries\n- Generate development-ready code\n\n**Requirements:**\n- Zeplin account with access to your project\n- Personal access token with appropriate permissions\n- Node.js installed (MCP server uses TypeScript)",
	},
	{
		id: "penpot",
		name: "Penpot MCP",
		description: "Open-source design & prototyping",
		icon: "palette",
		type: "local-http",
		localUrl: "http://localhost:4401/mcp",
		documentation:
			"Install Penpot MCP to connect Penpot designs to AI assistants.\n\n" +
			"Penpot MCP is an official local integration.\n\n" +
			"**Installation:**\n" +
			"Run: `npx -y @penpot/mcp@latest`\n\n" +
			"This will start both the MCP server and plugin server.\n\n" +
			"**Configuration:**\n" +
			"Add MCP connector in your AI client:\n" +
			"URL: http://localhost:4401/mcp\n\n" +
			"**Requirements:**\n" +
			"- Node.js v22.x\n" +
			"- Penpot account (web app or desktop app)\n" +
			"- Load Penpot MCP plugin in Penpot\n" +
			"- Keep both servers running\n" +
			"- MCP client that supports HTTP local servers\n\n" +
			"**Features:**\n" +
			"- Design-to-design workflows\n" +
			"- Code-to-design workflows\n" +
			"- Design-code supercharged workflows\n" +
			"- Design data queries and transformation\n\n" +
			"**Note:** Requires Penpot MCP plugin loaded and connected to MCP server.",
	},
	{
		id: "spline",
		name: "Spline MCP",
		description: "3D design & interactive experiences",
		icon: "box",
		type: "local-http",
		localUrl: "http://localhost:4401/mcp",
		documentation:
			"Install Spline MCP to connect Spline 3D to AI assistants.\n\n" +
			"Spline MCP is a local integration within Spline.\n\n" +
			"**Installation:**\n" +
			"1. Install and open Spline Desktop (Early Access)\n" +
			"2. Create or open a design file\n" +
			"3. Enable Spline MCP in the file menu\n" +
			"4. MCP server will start at: http://127.0.0.1:9791/mcp\n\n" +
			"**Configuration:**\n" +
			"Add MCP connector in your AI client:\n" +
			"URL: http://127.0.0.1:9791/mcp\n\n" +
			"**Requirements:**\n" +
			"- Spline Desktop (Windows or macOS)\n" +
			"- Spline Early Access\n" +
			"- Keep Spline Desktop open\n" +
			"- MCP client that supports HTTP local servers\n\n" +
			"**Features:**\n" +
			"- Copy layer link and paste in chat\n" +
			"- Select container on canvas\n" +
			"- Generate code from Spline designs\n\n" +
			"**Note:** Keep Spline Desktop open and the design file tab active for stable connection.",
	},
	{
		id: "canva",
		name: "Canva MCP",
		description: "Online design & publishing platform",
		icon: "image",
		type: "simple",
		config: {
			type: "local",
			command: ["npx", "-y", "@canva/cli", "mcp"],
		},
		envVars: [
			{
				key: "CANVA_API_KEY",
				label: "Canva API Key",
				helpUrl: "https://www.canva.dev/docs/mcp/",
			},
		],
		documentation:
			"Install Canva MCP to connect your Canva designs to AI assistants.\n\n**Installation:**\n`npx -y @canva/cli mcp`\n\n**Required Environment Variable:**\n- `CANVA_API_KEY`: Your Canva API key\n\n**Setup:**\n1. Go to Canva Developer Dashboard → Create App\n2. Generate API credentials\n3. Set the API key in your MCP configuration\n4. Ensure you're using stdio transport (MCP client requirement)\n\n**Usage:**\nOnce installed, the MCP server provides tools to:\n- Access Canva documentation and templates\n- Work with Canva apps and integrations\n- Fetch design information\n- Generate Canva-compatible assets\n\n**Requirements:**\n- Canva developer account\n- API key with appropriate permissions\n- MCP client that supports stdio transport\n\n**Note:** Canva MCP operates locally on your device, fetching documentation from canva.dev.",
	},
	{
		id: "rive",
		name: "Rive MCP",
		description: "Interactive animation runtime",
		icon: "play",
		type: "simple",
		config: {
			type: "local",
			command: ["npx", "-y", "@rive-mcp/server-core"],
		},
		documentation:
			"Install Rive MCP to control Rive animations from AI assistants.\n\n**Installation:**\n`npx -y @rive-mcp/server-core`\n\n**Requirements:**\n- Node.js 18+\n- npm 8+\n\n**Usage:**\nOnce installed, the MCP server provides tools to:\n- Inspect and validate Rive `.riv` animation files\n- Control Rive animations programmatically\n- Access Rive editor features and capabilities\n- Get runtime information for different platforms\n\n**Features:**\n- CLI tool for command-line operations\n- MCP server for AI integration\n- Visual playground for testing animations\n\n**Examples:**\n- Load and inspect `.riv` files\n- Validate animation integrity\n- Control animation playback\n- Extract metadata and information\n\n**Note:** This is an open-source MCP server maintained by the community.",
	},
]
