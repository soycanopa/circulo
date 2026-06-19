import type { InstallResult } from "../../preload/api"
import { useCallback, useState } from "react"

// ============================================================
// Types
// ============================================================

export type InstallState = "idle" | "installing" | "success" | "error"

// ============================================================
// Hook
// ============================================================

export function useSkillInstall() {
	const [state, setState] = useState<InstallState>("idle")
	const [error, setError] = useState<string | null>(null)
	const [currentSkill, setCurrentSkill] = useState<string | null>(null)

	const install = useCallback(async (ownerRepo: string, target?: string) => {
		if (!("circulo" in window)) return
		setState("installing")
		setCurrentSkill(ownerRepo)
		setError(null)

		try {
			const result: InstallResult = await window.circulo.skills.install({ ownerRepo, target })
			if (result.success) {
				setState("success")
			} else {
				setState("error")
				setError(result.error ?? "Installation failed")
			}
		} catch (err) {
			setState("error")
			setError(err instanceof Error ? err.message : "Installation failed")
		}
	}, [])

	const reset = useCallback(() => {
		setState("idle")
		setError(null)
		setCurrentSkill(null)
	}, [])

	return { state, error, currentSkill, install, reset }
}
