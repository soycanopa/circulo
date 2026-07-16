import { getDefaultStore } from "jotai"
import { recordProfilePrompt } from "@/lib/profile-activity"
import { setPromptInFlightSync } from "@/lib/prompt-flight"
import { sendPrompt } from "@/lib/tauri"
import { errorMessageAtom, pendingPromptAtom, promptInFlightAtom } from "@/stores/atoms"

export async function flushPendingPrompt(): Promise<boolean> {
	const store = getDefaultStore()
	const pending = store.get(pendingPromptAtom)
	if (!pending) return false

	store.set(pendingPromptAtom, null)

	try {
		await sendPrompt(pending.text, pending.contextPaths)
		recordProfilePrompt()
		return true
	} catch {
		store.set(promptInFlightAtom, false)
		setPromptInFlightSync(false)
		store.set(errorMessageAtom, "No se pudo enviar el mensaje")
		return false
	}
}