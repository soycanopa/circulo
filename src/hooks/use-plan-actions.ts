import { useAtom, useSetAtom } from "jotai"
import { useCallback } from "react"
import { normalizePlanMarkdown } from "@/lib/plan-markdown"
import { setPromptInFlightSync } from "@/lib/prompt-flight"
import { sendPrompt } from "@/lib/tauri"
import {
	pendingPlanAtom,
	planCommentModeAtom,
	promptInFlightAtom,
} from "@/stores/atoms"

export function downloadPlanMarkdown(content: string) {
	const blob = new Blob([content], { type: "text/markdown;charset=utf-8" })
	const url = URL.createObjectURL(blob)
	const anchor = document.createElement("a")
	anchor.href = url
	anchor.download = `plan-${new Date().toISOString().slice(0, 10)}.md`
	anchor.click()
	URL.revokeObjectURL(url)
}

export function usePlanActions() {
	const [pendingPlan, setPendingPlan] = useAtom(pendingPlanAtom)
	const setPlanCommentMode = useSetAtom(planCommentModeAtom)
	const setPromptInFlight = useSetAtom(promptInFlightAtom)

	const acceptPlan = useCallback(async () => {
		if (!pendingPlan?.content.trim()) return
		const plan = normalizePlanMarkdown(pendingPlan.content)
		setPendingPlan(null)
		setPromptInFlightSync(true)
		setPromptInFlight(true)
		try {
			await sendPrompt(
				"Acepto el plan. Procede con la implementación siguiendo este plan:\n\n" + plan,
				[],
			)
		} catch {
			setPromptInFlightSync(false)
			setPromptInFlight(false)
		}
	}, [pendingPlan, setPendingPlan, setPromptInFlight])

	const rejectPlan = useCallback(async () => {
		setPendingPlan(null)
		setPromptInFlightSync(true)
		setPromptInFlight(true)
		try {
			await sendPrompt("Rechazo el plan propuesto. No procedas con esa propuesta.", [])
		} catch {
			setPromptInFlightSync(false)
			setPromptInFlight(false)
		}
	}, [setPendingPlan, setPromptInFlight])

	const startPlanComment = useCallback(() => {
		setPlanCommentMode(true)
	}, [setPlanCommentMode])

	const downloadPlan = useCallback(() => {
		const content = pendingPlan?.content
		if (!content?.trim()) return
		downloadPlanMarkdown(content)
	}, [pendingPlan])

	const downloadContent = useCallback((content: string) => {
		if (!content.trim()) return
		downloadPlanMarkdown(content)
	}, [])

	return {
		pendingPlan,
		setPendingPlan,
		acceptPlan,
		rejectPlan,
		startPlanComment,
		downloadPlan,
		downloadContent,
	}
}