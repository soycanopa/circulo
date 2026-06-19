import type { InstalledSkill, SkillOrigin } from "../../preload/api"
import { useAtomValue } from "jotai"
import { useCallback, useEffect, useState } from "react"
import { discoveryProjectsAtom } from "../atoms/discovery"

// ============================================================
// Types
// ============================================================

export interface GroupedSkills {
	global: InstalledSkill[]
	project: InstalledSkill[]
	cursor: InstalledSkill[]
}

export interface ScanDiagnostics {
	globalDir: string
	agentsDir: string
	cursorDir: string
	projectDirs: string[]
	totalFound: number
}

// ============================================================
// Hook
// ============================================================

const SCANNED_DIRS = {
	global: "~/.config/opencode/skills/",
	agents: "~/.agents/skills/",
	cursor: "~/.cursor/skills/",
}

export function useInstalledSkills() {
	const [skills, setSkills] = useState<InstalledSkill[]>([])
	const [loading, setLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)
	const [diagnostics, setDiagnostics] = useState<ScanDiagnostics | null>(null)
	const projects = useAtomValue(discoveryProjectsAtom)

	const refresh = useCallback(async () => {
		console.info("[useInstalledSkills] refresh called", { hasCirculo: "circulo" in window })
		if (!("circulo" in window)) {
			setLoading(false)
			return
		}
		if (!window.circulo?.skills) {
			console.warn("[useInstalledSkills] window.circulo.skills not available")
			setError("Skills bridge not available")
			setLoading(false)
			return
		}
		setLoading(true)
		setError(null)
		try {
			const projectDirs: string[] = projects
				.filter(
					(p) =>
						"worktree" in p &&
						typeof (p as { worktree: unknown }).worktree === "string",
				)
				.map((p) => (p as { worktree: string }).worktree)
			const uniqueDirs = [...new Set(projectDirs)]
			console.info("[useInstalledSkills] calling skills.list", { projectDirs: uniqueDirs })
			const result = await window.circulo.skills.list(uniqueDirs)
			console.info("[useInstalledSkills] result", {
				total: result.length,
				global: result.filter((s) => s.origin === "global").length,
				project: result.filter((s) => s.origin === "project").length,
				cursor: result.filter((s) => s.origin === "cursor").length,
			})
			setSkills(result)
			setDiagnostics({
				globalDir: SCANNED_DIRS.global,
				agentsDir: SCANNED_DIRS.agents,
				cursorDir: SCANNED_DIRS.cursor,
				projectDirs: uniqueDirs,
				totalFound: result.length,
			})
		} catch (err) {
			const msg = err instanceof Error ? err.message : "Failed to load installed skills"
			console.error("[useInstalledSkills]", msg, err)
			setError(msg)
		} finally {
			setLoading(false)
		}
	}, [projects])

	useEffect(() => {
		refresh()
	}, [refresh])

	const grouped: GroupedSkills = {
		global: skills
			.filter((s) => s.origin === "global")
			.sort((a, b) => a.name.localeCompare(b.name)),
		project: skills
			.filter((s) => s.origin === "project")
			.sort((a, b) => a.name.localeCompare(b.name)),
		cursor: skills
			.filter((s) => s.origin === "cursor")
			.sort((a, b) => a.name.localeCompare(b.name)),
	}

	const projectsMap = new Map<string, InstalledSkill[]>()
	for (const skill of skills) {
		if (skill.project) {
			const list = projectsMap.get(skill.project) ?? []
			list.push(skill)
			projectsMap.set(skill.project, list)
		}
	}
	for (const [key, list] of projectsMap) {
		projectsMap.set(
			key,
			list.sort((a, b) => a.name.localeCompare(b.name)),
		)
	}

	return { skills, grouped, projectsMap, loading, error, refresh, diagnostics }
}

export type { InstalledSkill, SkillOrigin }
