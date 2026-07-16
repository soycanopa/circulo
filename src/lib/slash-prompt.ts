import type {
	OpencodeCommandEntry,
	OpencodeMcpServerEntry,
	OpencodeSkillEntry,
} from "@/lib/tauri"

export type SlashEntryKind = "command" | "skill" | "mcp"

export interface SlashEntry {
	name: string
	description: string | null
	scope: string
	kind: SlashEntryKind
}

const SLASH_INVOKE_PATTERN = /^\/([^\s/]+)(?:\s+([\s\S]*))?$/i

export function extractSlashQuery(value: string, caret: number): string | null {
	const beforeCaret = value.slice(0, caret)
	const match = /(?:^|\s)\/([^\s/]*)$/.exec(beforeCaret)
	return match ? match[1] : null
}

function formatMcpDescription(entry: OpencodeMcpServerEntry): string | null {
	const parts = [entry.serverType, entry.scope].filter(Boolean)
	return parts.length > 0 ? parts.join(" · ") : null
}

export function buildSlashEntries(
	commands: OpencodeCommandEntry[],
	skills: OpencodeSkillEntry[],
	mcpServers: OpencodeMcpServerEntry[],
): SlashEntry[] {
	const commandEntries: SlashEntry[] = commands.map((entry) => ({
		name: entry.name,
		description: entry.description,
		scope: entry.scope,
		kind: "command",
	}))
	const skillEntries: SlashEntry[] = skills.map((entry) => ({
		name: entry.name,
		description: entry.description,
		scope: entry.scope,
		kind: "skill",
	}))
	const mcpEntries: SlashEntry[] = mcpServers
		.filter((entry) => entry.enabled)
		.map((entry) => ({
			name: entry.name,
			description: formatMcpDescription(entry),
			scope: entry.scope,
			kind: "mcp",
		}))
	return [...commandEntries, ...skillEntries, ...mcpEntries].sort((a, b) =>
		a.name.localeCompare(b.name),
	)
}

export function filterSlashEntries(entries: SlashEntry[], query: string): SlashEntry[] {
	const normalized = query.trim().toLowerCase()
	if (!normalized) return entries
	return entries.filter((entry) => entry.name.toLowerCase().startsWith(normalized))
}

export function insertSlashEntry(value: string, caret: number, entry: SlashEntry): string {
	const beforeCaret = value.slice(0, caret)
	const afterCaret = value.slice(caret)
	const replaced = beforeCaret.replace(/\/([^\s/]*)$/, `/${entry.name} `)
	return `${replaced}${afterCaret}`
}

function parseSlashInvoke(text: string): { name: string; args: string | null } | null {
	const match = SLASH_INVOKE_PATTERN.exec(text.trim())
	if (!match) return null
	return { name: match[1], args: match[2]?.trim() ?? null }
}

function findByName<T extends { name: string; scope: string }>(
	entries: T[],
	name: string,
): T | null {
	const normalized = name.toLowerCase()
	const project = entries.find(
		(entry) => entry.scope === "project" && entry.name.toLowerCase() === normalized,
	)
	if (project) return project
	return entries.find((entry) => entry.name.toLowerCase() === normalized) ?? null
}

function expandSkillPrompt(skill: OpencodeSkillEntry, args: string | null): string {
	if (args) {
		return `Use the "${skill.name}" skill for this task:\n\n${args}`
	}
	return `Use the "${skill.name}" skill for this task.`
}

function expandMcpPrompt(mcp: OpencodeMcpServerEntry, args: string | null): string {
	if (args) {
		return `Use the "${mcp.name}" MCP server and its tools for this task:\n\n${args}`
	}
	return `Use the "${mcp.name}" MCP server and its tools for this task.`
}

/** OpenCode commands pass through; skills and MCPs expand to agent-friendly prompts. */
export function expandSlashPrompt(
	text: string,
	commands: OpencodeCommandEntry[],
	skills: OpencodeSkillEntry[],
	mcpServers: OpencodeMcpServerEntry[],
): string {
	const trimmed = text.trim()
	if (!trimmed.startsWith("/")) return text

	const invoke = parseSlashInvoke(trimmed)
	if (!invoke) return trimmed

	if (findByName(commands, invoke.name)) {
		return trimmed
	}

	const skill = findByName(skills, invoke.name)
	if (skill) {
		return expandSkillPrompt(skill, invoke.args)
	}

	const mcp = findByName(
		mcpServers.filter((entry) => entry.enabled),
		invoke.name,
	)
	if (mcp) {
		return expandMcpPrompt(mcp, invoke.args)
	}

	return trimmed
}