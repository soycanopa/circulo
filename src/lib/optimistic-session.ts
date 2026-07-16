import type { SessionInfo } from "@/types/acp"

export const OPTIMISTIC_SESSION_ID = "__optimistic_session__"

export function isOptimisticSessionId(sessionId: string | null | undefined): boolean {
	return sessionId === OPTIMISTIC_SESSION_ID
}

export function createOptimisticSessionEntry(projectPath: string): SessionInfo {
	return {
		sessionId: OPTIMISTIC_SESSION_ID,
		cwd: projectPath,
		additionalDirectories: [],
		title: "Nueva sesión",
		updatedAt: new Date().toISOString(),
	}
}