import { useAtom, useSetAtom } from "jotai"
import { useCallback } from "react"
import { normalizePlanMarkdown } from "@/lib/plan-markdown"
import { setPromptInFlightSync } from "@/lib/prompt-flight"
import { recordProfilePrompt } from "@/lib/profile-activity"
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

	const submitAcceptedPlan = useCallback(
		async (compactFirst: boolean) => {
			if (!pendingPlan?.content.trim()) return
			const plan = normalizePlanMarkdown(pendingPlan.content)
			setPendingPlan(null)
			setPromptInFlightSync(true)
			setPromptInFlight(true)
			const acceptBody =
				"Acepto el plan. Procede con la implementación siguiendo este plan:\n\n" + plan
			const prompt = compactFirst ? `/compact\n\n${acceptBody}` : acceptBody
			try {
				await sendPrompt(prompt, [])
				recordProfilePrompt()
			} catch {
				setPromptInFlightSync(false)
				setPromptInFlight(false)
			}
		},
		[pendingPlan, setPendingPlan, setPromptInFlight],
	)

	const acceptPlan = useCallback(async () => {
		await submitAcceptedPlan(false)
	}, [submitAcceptedPlan])

	const acceptAndCompactPlan = useCallback(async () => {
		await submitAcceptedPlan(true)
	}, [submitAcceptedPlan])

	const rejectPlan = useCallback(async () => {
		setPendingPlan(null)
		setPromptInFlightSync(true)
		setPromptInFlight(true)
		try {
			await sendPrompt("Rechazo el plan propuesto. No procedas con esa propuesta.", [])
			recordProfilePrompt()
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
		acceptAndCompactPlan,
		rejectPlan,
		startPlanComment,
		downloadPlan,
		downloadContent,
	}
}