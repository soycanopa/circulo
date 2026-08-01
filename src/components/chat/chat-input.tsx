import { useAtomValue, useSetAtom } from "jotai"
import { CornerDownLeft, Loader2 } from "lucide-react"
import { useState } from "react"
import { PermissionPrompt } from "@/components/permissions/permission-prompt"
import { sendPrompt } from "@/lib/tauri"
import {
	activePermissionAtom,
	errorMessageAtom,
	messagesAtom,
	promptInFlightAtom,
	sessionIdAtom,
	sessionStatusAtom,
} from "@/stores/atoms"

export function ChatInput() {
	const sessionId = useAtomValue(sessionIdAtom)
	const status = useAtomValue(sessionStatusAtom)
	const promptInFlight = useAtomValue(promptInFlightAtom)
	const permission = useAtomValue(activePermissionAtom)
	const setMessages = useSetAtom(messagesAtom)
	const setPromptInFlight = useSetAtom(promptInFlightAtom)
	const setStatus = useSetAtom(sessionStatusAtom)
	const setError = useSetAtom(errorMessageAtom)
	const [value, setValue] = useState("")

	const disabled =
		!sessionId ||
		promptInFlight ||
		status === "connecting" ||
		status === "awaiting_permission" ||
		Boolean(permission)

	async function handleSubmit(event?: React.FormEvent) {
		event?.preventDefault()
		const trimmed = value.trim()
		if (!trimmed || disabled) return

		// Optimistic user + empty assistant so the stream bubble is instant (Palot-like).
		setMessages((current) => [
			...current,
			{
				id: crypto.randomUUID(),
				role: "user",
				content: trimmed,
				toolCalls: [],
				timestamp: Date.now(),
			},
			{
				id: crypto.randomUUID(),
				role: "assistant",
				content: "",
				toolCalls: [],
				timestamp: Date.now(),
			},
		])
		setValue("")
		setPromptInFlight(true)
		setStatus("generating")
		setError(null)

		try {
			// send_prompt queues and returns; tokens arrive via session/update events.
			await sendPrompt(trimmed, [])
		} catch (error) {
			setPromptInFlight(false)
			setStatus("idle")
			setError(error instanceof Error ? error.message : "Failed to send prompt")
		}
	}

	return (
		<div className="shrink-0 border-t border-border px-4 py-3">
			<div className="mx-auto max-w-3xl">
				<PermissionPrompt />
				<form
					onSubmit={(e) => void handleSubmit(e)}
					className="rounded-lg border border-border bg-surface focus-within:border-white/20"
				>
					<textarea
						value={value}
						onChange={(e) => setValue(e.target.value)}
						onKeyDown={(e) => {
							if (e.key === "Enter" && !e.shiftKey) {
								e.preventDefault()
								void handleSubmit()
							}
						}}
						disabled={disabled}
						rows={3}
						placeholder={
							!sessionId
								? "New Chat first to open a session…"
								: permission
									? "Respond to the permission request…"
									: "Message the agent…"
						}
						className="w-full resize-none bg-transparent px-3 py-2.5 text-sm text-fg outline-none placeholder:text-muted disabled:opacity-50"
					/>
					<div className="flex items-center justify-end border-t border-border px-2 py-1.5">
						<button
							type="submit"
							disabled={disabled || !value.trim()}
							className="inline-flex items-center gap-1.5 rounded-md bg-white/10 px-2.5 py-1 text-xs font-medium text-fg transition hover:bg-white/15 disabled:opacity-40"
						>
							{promptInFlight ? (
								<Loader2 className="size-3.5 animate-spin" />
							) : (
								<CornerDownLeft className="size-3.5" />
							)}
							Send
						</button>
					</div>
				</form>
			</div>
		</div>
	)
}
