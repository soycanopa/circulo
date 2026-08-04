import { useCallback, useEffect, useState } from "react"
import { listAutomations } from "@/lib/tauri"
import type { Automation } from "@/types/acp"

export function useAutomations() {
	const [automations, setAutomations] = useState<Automation[]>([])

	const refresh = useCallback(async () => {
		try {
			const items = await listAutomations()
			setAutomations(items)
		} catch {
			setAutomations([])
		}
	}, [])

	useEffect(() => {
		void refresh()
	}, [refresh])

	return { automations, refresh }
}
