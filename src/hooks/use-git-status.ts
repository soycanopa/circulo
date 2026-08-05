import { listen } from "@tauri-apps/api/event"
import { useCallback, useEffect, useRef, useState } from "react"
import { getGitStatus } from "@/lib/tauri"
import type { GitStatus } from "@/types/acp"

interface UseGitStatusResult {
	status: GitStatus | null
	loading: boolean
	isRepo: boolean
	error: string | null
	refresh: () => void
}

const REFRESH_DEBOUNCE_MS = 800

/**
 * Working-tree git status for the open project. Reloads when the project
 * changes and after the agent completes a prompt (files may have changed).
 */
export function useGitStatus(
	projectPath: string | null,
): UseGitStatusResult {
	const [status, setStatus] = useState<GitStatus | null>(null)
	const [loading, setLoading] = useState(false)
	const [isRepo, setIsRepo] = useState(true)
	const [error, setError] = useState<string | null>(null)
	const inFlight = useRef(false)
	const debounceRef = useRef<number | null>(null)

	const load = useCallback(async (path: string) => {
		if (inFlight.current) return
		inFlight.current = true
		setLoading(true)
		try {
			const result = await getGitStatus(path)
			setStatus(result)
			setIsRepo(true)
			setError(null)
		} catch (err) {
			setStatus(null)
			setIsRepo(false)
			setError(err instanceof Error ? err.message : "git status failed")
		} finally {
			inFlight.current = false
			setLoading(false)
		}
	}, [])

	// Reload whenever the project changes.
	useEffect(() => {
		setStatus(null)
		if (!projectPath) {
			setIsRepo(false)
			return
		}
		void load(projectPath)
	}, [projectPath, load])

	// Debounced refresh after each completed agent turn.
	useEffect(() => {
		let unlisten: (() => void) | undefined
		listen("acp:prompt_complete", () => {
			if (debounceRef.current) window.clearTimeout(debounceRef.current)
			debounceRef.current = window.setTimeout(() => {
				if (projectPath) void load(projectPath)
			}, REFRESH_DEBOUNCE_MS)
		}).then((fn) => {
			unlisten = fn
		})
		return () => {
			unlisten?.()
			if (debounceRef.current) window.clearTimeout(debounceRef.current)
		}
	}, [projectPath, load])

	const refresh = useCallback(() => {
		if (projectPath) void load(projectPath)
	}, [projectPath, load])

	return { status, loading, isRepo, error, refresh }
}
