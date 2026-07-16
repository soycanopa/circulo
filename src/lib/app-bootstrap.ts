import { getAppSettings, getChatsProjectPath } from "@/lib/app-settings"
import { getProjectStatus, openProject } from "@/lib/tauri"
import type { ProjectStatus } from "@/lib/tauri"

const LAST_PROJECT_KEY = "circulo-last-project"

let bootstrapPromise: Promise<ProjectStatus> | null = null

export function getLastProjectPath(): string | null {
	try {
		const path = localStorage.getItem(LAST_PROJECT_KEY)
		return path?.trim() ? path : null
	} catch {
		return null
	}
}

export function setLastProjectPath(path: string | null): void {
	try {
		if (!path) {
			localStorage.removeItem(LAST_PROJECT_KEY)
			return
		}
		localStorage.setItem(LAST_PROJECT_KEY, path)
	} catch {
		// ignore storage failures
	}
}

async function resolveStartupStatus(): Promise<ProjectStatus> {
	const current = await getProjectStatus()
	if (current.connected) return current

	try {
		return await openProject(getChatsProjectPath(), {
			agentId: getAppSettings().defaultProvider,
			deferSessionBootstrap: true,
		})
	} catch {
		return current
	}
}

/** Runs once per app load — avoids StrictMode double-spawn of the OpenCode agent. */
export function bootstrapAppStatus(): Promise<ProjectStatus> {
	if (!bootstrapPromise) {
		bootstrapPromise = resolveStartupStatus()
	}
	return bootstrapPromise
}