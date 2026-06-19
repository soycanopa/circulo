import { exec } from "node:child_process"
import { existsSync, readdirSync, readFileSync, rmSync } from "node:fs"
import path from "node:path"
import { homedir } from "node:os"
import { createLogger } from "./logger"

const log = createLogger("skills")

// ============================================================
// Types
// ============================================================

export type SkillOrigin = "global" | "project" | "cursor"

export interface InstalledSkill {
	name: string
	description: string
	location: string
	origin: SkillOrigin
	project?: string
}

export interface InstallSkillParams {
	ownerRepo: string
	skillName?: string
	target?: string
}

export interface InstallResult {
	success: boolean
	error?: string
}

export interface RemoveResult {
	success: boolean
	error?: string
}

// ============================================================
// Metadata extraction
// ============================================================

/**
 * Reads a skill's SKILL.md and extracts name/description.
 * Tries YAML frontmatter first, falls back to directory name and first content line.
 */
function extractSkillMetadata(skillDir: string, dirName: string): { name: string; description: string } {
	const skillMdPath = path.join(skillDir, "SKILL.md")
	if (!existsSync(skillMdPath)) {
		return { name: dirName, description: "No description available" }
	}

	try {
		const content = readFileSync(skillMdPath, "utf-8")
		const name = dirName

		// Try YAML frontmatter for description
		const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---/)
		if (frontmatterMatch) {
			const fm = frontmatterMatch[1]
			const descMatch = fm.match(/^description:\s*(.+)$/m)
			if (descMatch) {
				return { name, description: descMatch[1].trim() }
			}
		}

		// Fallback: first non-empty, non-heading line
		const lines = content.split("\n")
		for (const line of lines) {
			const trimmed = line.trim()
			if (trimmed && !trimmed.startsWith("#") && !trimmed.startsWith("---")) {
				return { name, description: trimmed.slice(0, 200) }
			}
		}

		return { name, description: "No description available" }
	} catch {
		return { name: dirName, description: "No description available" }
	}
}

// ============================================================
// Directory scanning
// ============================================================

function scanDirectory(
	skillsDir: string,
	origin: SkillOrigin,
	project?: string,
): InstalledSkill[] {
	if (!existsSync(skillsDir)) return []

	try {
		const entries = readdirSync(skillsDir, { withFileTypes: true })
		const skills: InstalledSkill[] = []

		for (const entry of entries) {
			if (!entry.isDirectory()) continue
			const skillDir = path.join(skillsDir, entry.name)

			// If this directory has a SKILL.md, it's a skill
			if (existsSync(path.join(skillDir, "SKILL.md"))) {
				const { name, description } = extractSkillMetadata(skillDir, entry.name)
				skills.push({
					name,
					description,
					location: skillDir,
					origin,
					...(project ? { project } : {}),
				})
			} else {
				// No SKILL.md at this level — scan one level deeper for nested skills
				try {
					const nested = readdirSync(skillDir, { withFileTypes: true })
					for (const nestedEntry of nested) {
						if (!nestedEntry.isDirectory()) continue
						const nestedDir = path.join(skillDir, nestedEntry.name)
						if (existsSync(path.join(nestedDir, "SKILL.md"))) {
							const { name, description } = extractSkillMetadata(nestedDir, nestedEntry.name)
							skills.push({
								name,
								description,
								location: nestedDir,
								origin,
								...(project ? { project } : {}),
							})
						}
					}
				} catch {
					// Ignore nested scan errors
				}
			}
		}

		log.info(`  ${skillsDir}: ${skills.length} skills found`)
		return skills
	} catch (err) {
		log.warn(`Failed to scan skills directory: ${skillsDir}`, err)
		return []
	}
}

// ============================================================
// Public API
// ============================================================

export function scanSkills(registeredProjectDirs: string[] = []): InstalledSkill[] {
	const skills: InstalledSkill[] = []

	// Global skills
	const globalDir = path.join(homedir(), ".config", "opencode", "skills")
	log.info(`Scanning global skills at ${globalDir}`)
	skills.push(...scanDirectory(globalDir, "global"))

	// Cursor-migrated skills
	const cursorDir = path.join(homedir(), ".cursor", "skills")
	log.info(`Scanning cursor skills at ${cursorDir}`)
	skills.push(...scanDirectory(cursorDir, "cursor"))

	// Global agents skills (~/.agents/skills/ — used by Cline, Codex, Cursor, Copilot, etc.)
	const agentsDir = path.join(homedir(), ".agents", "skills")
	log.info(`Scanning global agents skills at ${agentsDir}`)
	skills.push(...scanDirectory(agentsDir, "global"))

	// Project-local skills
	for (const projectDir of registeredProjectDirs) {
		log.info(`Scanning project skills for ${projectDir}`)
		skills.push(
			...scanDirectory(path.join(projectDir, ".agents", "skills"), "project", projectDir),
		)
		skills.push(
			...scanDirectory(path.join(projectDir, ".opencode", "skills"), "project", projectDir),
		)
	}

	log.info(`Found ${skills.length} skills total`)
	return skills
}

// ============================================================
// Installation queue
// ============================================================

let installQueue: Promise<void> = Promise.resolve()

function enqueueInstall<T>(fn: () => Promise<T>): Promise<T> {
	const prev = installQueue
	let resolveEnqueue: () => void
	installQueue = new Promise<void>((resolve) => {
		resolveEnqueue = resolve
	})

	return prev.then(async () => {
		try {
			return await fn()
		} finally {
			resolveEnqueue!()
		}
	})
}

// ============================================================
// Skill installation
// ============================================================

function execCommand(
	command: string,
	timeoutMs: number,
	cwd?: string,
): Promise<{ stdout: string; stderr: string }> {
	return new Promise((resolve, reject) => {
		exec(command, { timeout: timeoutMs, cwd }, (error, stdout, stderr) => {
			if (error) {
				reject(new Error(stderr || error.message))
			} else {
				resolve({ stdout, stderr })
			}
		})
	})
}

export function installSkill(params: InstallSkillParams): Promise<InstallResult> {
	return enqueueInstall(async () => {
		const { ownerRepo, skillName, target } = params

		const skillFlag = skillName ? ` --skill "${skillName}"` : ""

		if (target) {
			// Per-project install: run in project dir with -y (non-interactive)
			const cmd = `npx skills add ${ownerRepo}${skillFlag} -y`
			try {
				await execCommand(cmd, 60000, target)
				log.info(`Installed skill ${skillName ?? ownerRepo} to project ${target}`)
				return { success: true }
			} catch (err) {
				const msg = err instanceof Error ? err.message : String(err)
				log.error(`Failed to install skill ${ownerRepo} to ${target}: ${msg}`)
				return { success: false, error: msg }
			}
		}

		// Global install via npx with -g -y (global, non-interactive)
		const cmd = `npx skills add ${ownerRepo}${skillFlag} -g -y`
		try {
			const { stderr } = await execCommand(cmd, 60000)
			log.info(`Installed skill ${skillName ?? ownerRepo} globally`)
			if (stderr && !stderr.includes("WARN")) {
				log.warn(`Install stderr: ${stderr}`)
			}
			return { success: true }
		} catch (err) {
			const msg = err instanceof Error ? err.message : String(err)
			log.error(`Failed to install skill ${ownerRepo}: ${msg}`)
			return { success: false, error: msg }
		}
	})
}

/**
 * Removes a skill directory from disk.
 */
export function removeSkill(skillPath: string): RemoveResult {
	if (!existsSync(skillPath)) {
		return { success: false, error: `Skill path not found: ${skillPath}` }
	}
	try {
		rmSync(skillPath, { recursive: true })
		log.info(`Removed skill at ${skillPath}`)
		return { success: true }
	} catch (err) {
		const msg = err instanceof Error ? err.message : String(err)
		log.error(`Failed to remove skill at ${skillPath}: ${msg}`)
		return { success: false, error: msg }
	}
}

/**
 * Derives unique project directories from the list of registered OpenCode projects
 * (obtained from the renderer's discovery atom). Called via IPC.
 */
export function getRegisteredProjects(): string[] {
	// We scan the home directory's opencode projects directory
	// for registered project paths.
	const projectsDir = path.join(homedir(), ".config", "opencode", "projects")
	if (!existsSync(projectsDir)) return []

	try {
		return readdirSync(projectsDir, { withFileTypes: true })
			.filter((e) => e.isDirectory())
			.map((e) => path.join(projectsDir, e.name))
	} catch {
		return []
	}
}
