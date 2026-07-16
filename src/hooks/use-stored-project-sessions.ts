import { useEffect, useMemo, useState } from "react"
import { listStoredSessions } from "@/lib/tauri"
import type { SessionInfo } from "@/types/acp"

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
					const sessions = await listStoredSessions(path)
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