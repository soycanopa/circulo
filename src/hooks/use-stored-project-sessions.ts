import { useEffect, useMemo, useState } from "react"
import { listStoredSessions, prefetchProjectSessions } from "@/lib/tauri"
import type { SessionInfo } from "@/types/acp"

async function loadSessionsForProject(path: string): Promise<SessionInfo[]> {
	try {
		const prefetched = await prefetchProjectSessions(path)
		if (prefetched.length > 0) return prefetched
	} catch {
		// CLI prefetch is best-effort; fall back to Circulo store.
	}
	return listStoredSessions(path)
}

export function useStoredProjectSessions(projectPaths: string[]) {
	const [storedByProject, setStoredByProject] = useState<Record<string, SessionInfo[]>>({})
	const pathsKey = useMemo(() => projectPaths.join("\0"), [projectPaths])

	useEffect(() => {
		if (projectPaths.length === 0) {
			setStoredByProject({})
			return
		}

		let cancelled = false

		void Promise.all(
			projectPaths.map(async (path) => {
				try {
					const sessions = await loadSessionsForProject(path)
					return [path, sessions] as const
				} catch {
					return [path, []] as const
				}
			}),
		).then((entries) => {
			if (cancelled) return
			setStoredByProject(Object.fromEntries(entries))
		})

		return () => {
			cancelled = true
		}
	}, [pathsKey, projectPaths])

	return { storedByProject, setStoredByProject }
}