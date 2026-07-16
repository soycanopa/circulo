import type { OpencodeCommandEntry, OpencodeSkillEntry } from "@/lib/tauri"

export type SlashEntryKind = "command" | "skill"

export interface SlashEntry {
	name: string
	description: string | null
	scope: string
	kind: SlashEntryKind
}

const SLASH_NAME_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/i

export function extractSlashQuery(value: string, caret: number): string | null {
	const beforeCaret = value.slice(0, caret)
	const match = /(?:^|\s)\/([^\s/]*)$/.exec(beforeCaret)
	return match ? match[1] : null
}

export function buildSlashEntries(
	commands: OpencodeCommandEntry[],
	skills: OpencodeSkillEntry[],
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
	return [...commandEntries, ...skillEntries].sort((a, b) => a.name.localeCompare(b.name))
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

function findCommandName(
	text: string,
	commands: OpencodeCommandEntry[],
): OpencodeCommandEntry | null {
	const match = /^\/([a-z0-9]+(?:-[a-z0-9]+)*)(?:\s|$)/i.exec(text.trim())
	if (!match) return null
	const name = match[1].toLowerCase()
	const project = commands.find(
		(entry) => entry.scope === "project" && entry.name.toLowerCase() === name,
	)
	if (project) return project
	return commands.find((entry) => entry.name.toLowerCase() === name) ?? null
}

function findSkillName(text: string, skills: OpencodeSkillEntry[]): OpencodeSkillEntry | null {
	const match = /^\/([a-z0-9]+(?:-[a-z0-9]+)*)(?:\s+([\s\S]*))?$/i.exec(text.trim())
	if (!match || !SLASH_NAME_PATTERN.test(match[1])) return null
	const name = match[1].toLowerCase()
	const project = skills.find(
		(entry) => entry.scope === "project" && entry.name.toLowerCase() === name,
	)
	if (project) return project
	return skills.find((entry) => entry.name.toLowerCase() === name) ?? null
}

/** OpenCode commands pass through; skills expand to an agent-friendly prompt. */
export function expandSlashPrompt(
	text: string,
	commands: OpencodeCommandEntry[],
	skills: OpencodeSkillEntry[],
): string {
	const trimmed = text.trim()
	if (!trimmed.startsWith("/")) return text

	if (findCommandName(trimmed, commands)) {
		return trimmed
	}

	const skillMatch = /^\/([a-z0-9]+(?:-[a-z0-9]+)*)(?:\s+([\s\S]*))?$/i.exec(trimmed)
	if (!skillMatch) return trimmed

	const skill = findSkillName(trimmed, skills)
	if (!skill) return trimmed

	const args = skillMatch[2]?.trim()
	if (args) {
		return `Use the "${skill.name}" skill for this task:\n\n${args}`
	}
	return `Use the "${skill.name}" skill for this task.`
}