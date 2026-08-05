import { useAtomValue } from "jotai"
import { useCallback, useEffect, useState } from "react"
import { getGitBranches } from "@/lib/tauri"
import { gitRefreshVersionAtom } from "@/stores/atoms"
import type { GitBranches } from "@/types/acp"

interface UseGitBranchesResult {
	branches: GitBranches | null
	loading: boolean
	isRepo: boolean
	error: string | null
	refresh: () => void
}

/**
 * Git branch list for the open project. Reloads when the project changes,
 * when a git operation bumps `gitRefreshVersionAtom`, and on demand via
 * `refresh()`.
 */
export function useGitBranches(
	projectPath: string | null,
): UseGitBranchesResult {
	const [branches, setBranches] = useState<GitBranches | null>(null)
	const [loading, setLoading] = useState(false)
	const [isRepo, setIsRepo] = useState(true)
	const [error, setError] = useState<string | null>(null)
	const gitRefreshVersion = useAtomValue(gitRefreshVersionAtom)

	const load = useCallback(async (path: string) => {
		setLoading(true)
		try {
			const result = await getGitBranches(path)
			setBranches(result)
			setIsRepo(true)
			setError(null)
		} catch (err) {
			setBranches(null)
			setIsRepo(false)
			setError(err instanceof Error ? err.message : "git branches failed")
		} finally {
			setLoading(false)
		}
	}, [])

	useEffect(() => {
		setBranches(null)
		if (!projectPath) {
			setIsRepo(false)
			return
		}
		void load(projectPath)
	}, [projectPath, load])

	useEffect(() => {
		if (projectPath && gitRefreshVersion > 0) void load(projectPath)
	}, [gitRefreshVersion, projectPath, load])

	const refresh = useCallback(() => {
		if (projectPath) void load(projectPath)
	}, [projectPath, load])

	return { branches, loading, isRepo, error, refresh }
}
