const PROJECT_ALIASES_KEY = "circulo-project-aliases"

type ProjectAliasMap = Record<string, string>

function readAliases(): ProjectAliasMap {
	try {
		const raw = localStorage.getItem(PROJECT_ALIASES_KEY)
		if (!raw) return {}
		const parsed: unknown = JSON.parse(raw)
		if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {}
		return Object.fromEntries(
			Object.entries(parsed).filter(
				(entry): entry is [string, string] =>
					typeof entry[0] === "string" && typeof entry[1] === "string",
			),
		)
	} catch {
		return {}
	}
}

function writeAliases(aliases: ProjectAliasMap): void {
	localStorage.setItem(PROJECT_ALIASES_KEY, JSON.stringify(aliases))
}

export function getProjectAlias(path: string): string | null {
	const alias = readAliases()[path]?.trim()
	return alias || null
}

export function setProjectAlias(path: string, alias: string): void {
	const trimmed = alias.trim()
	const aliases = readAliases()
	if (!trimmed) {
		delete aliases[path]
	} else {
		aliases[path] = trimmed
	}
	writeAliases(aliases)
}

export function removeProjectAlias(path: string): void {
	const aliases = readAliases()
	delete aliases[path]
	writeAliases(aliases)
}

export function getProjectSidebarLabel(path: string): string {
	return getProjectAlias(path) ?? path.split("/").pop() ?? path
}