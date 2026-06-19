import { useCallback, useRef, useState } from "react"

// ============================================================
// Types (matches https://skills.sh/api/search response)
// ============================================================

export interface DiscoveredSkill {
	id: string
	name: string
	source: string
	installs: number
	url: string
}

interface ApiSearchResult {
	id: string
	skillId: string
	name: string
	installs: number
	source: string
}

interface ApiResponse {
	query: string
	searchType: string
	skills: ApiSearchResult[]
	count: number
}

// ============================================================
// Hook
// ============================================================

export function useSkillDiscovery() {
	const [skills, setSkills] = useState<DiscoveredSkill[]>([])
	const [loading, setLoading] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const [lastQuery, setLastQuery] = useState("")
	const abortRef = useRef<AbortController | null>(null)

	const doSearch = async (q: string) => {
		if (!q.trim() || q.trim().length < 2) return

		abortRef.current?.abort()
		const controller = new AbortController()
		abortRef.current = controller

		setLoading(true)
		setError(null)
		setLastQuery(q)
		setSkills([])

		try {
			const res = await fetch(
				`https://skills.sh/api/search?q=${encodeURIComponent(q)}&limit=50`,
				{ signal: controller.signal },
			)
			if (!res.ok) throw new Error(`Search failed: ${res.status}`)
			const data: ApiResponse = await res.json()
			const mapped: DiscoveredSkill[] = (data.skills ?? []).map((s) => ({
				id: s.id,
				name: s.name,
				source: s.source,
				installs: s.installs ?? 0,
				url: `https://skills.sh/${s.id}`,
			}))
			if (!controller.signal.aborted) {
				setSkills(mapped)
			}
		} catch (err) {
			if (err instanceof DOMException && err.name === "AbortError") return
			setError(err instanceof Error ? err.message : "Search failed")
		} finally {
			if (!controller.signal.aborted) {
				setLoading(false)
			}
		}
	}

	const fetchInitial = useCallback(() => {
		doSearch("development")
	}, [])

	const openSkillsSh = useCallback(() => {
		window.open("https://skills.sh", "_blank")
	}, [])

	return { skills, loading, error, lastQuery, search: doSearch, fetchInitial, openSkillsSh }
}
