import { useAtomValue, useSetAtom } from "jotai"
import { CornerDownLeft, Loader2, Square } from "lucide-react"
import { useRef, useState } from "react"
import { FileMentionPicker } from "@/components/chat/file-mention-picker"
import { PermissionPrompt } from "@/components/permissions/permission-prompt"
import {
	extractMentionPaths,
	getActiveMention,
	insertMention,
} from "@/lib/mention-parser"
import { cancelPrompt, sendPrompt } from "@/lib/tauri"
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
	const [mentionIndex, setMentionIndex] = useState(0)
	const [mentionResults, setMentionResults] = useState<string[]>([])
	const textareaRef = useRef<HTMLTextAreaElement>(null)

	const disabled =
		!sessionId ||
		promptInFlight ||
		status === "connecting" ||
		status === "awaiting_permission" ||
		Boolean(permission)

	const cursor = textareaRef.current?.selectionStart ?? value.length
	const activeMention = getActiveMention(value, cursor)
	const showMentionPicker = Boolean(activeMention && sessionId && !disabled)

	function updateValue(next: string, nextCursor: number) {
		setValue(next)
		requestAnimationFrame(() => {
			const el = textareaRef.current
			if (!el) return
			el.focus()
			el.setSelectionRange(nextCursor, nextCursor)
		})
	}

	function applyMention(path: string) {
		if (!activeMention) return
		const { value: next, cursor: nextCursor } = insertMention(
			value,
			activeMention.start,
			cursor,
			path,
		)
		updateValue(next, nextCursor)
		setMentionIndex(0)
		setMentionResults([])
	}

	function selectHighlightedMention(): boolean {
		if (!showMentionPicker || mentionResults.length === 0) return false
		const path =
			mentionResults[mentionIndex % mentionResults.length] ?? mentionResults[0]
		if (!path) return false
		applyMention(path)
		return true
	}

	async function handleSubmit(event?: React.FormEvent) {
		event?.preventDefault()
		const trimmed = value.trim()
		if (!trimmed || disabled) return

		const contextPaths = extractMentionPaths(trimmed)

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
		setMentionIndex(0)
		setMentionResults([])
		setPromptInFlight(true)
		setStatus("generating")
		setError(null)

		try {
			await sendPrompt(trimmed, contextPaths)
		} catch (error) {
			setPromptInFlight(false)
			setStatus("idle")
			setError(error instanceof Error ? error.message : "Failed to send prompt")
		}
	}

	async function handleCancel() {
		try {
			await cancelPrompt()
		} catch (error) {
			setError(error instanceof Error ? error.message : "Failed to cancel")
		}
	}

	return (
		<div className="shrink-0 border-t border-border px-4 py-3">
			<div className="mx-auto max-w-3xl">
				<PermissionPrompt />
				<form
					onSubmit={(e) => void handleSubmit(e)}
					className="relative rounded-lg border border-border bg-surface focus-within:border-white/20"
				>
					{showMentionPicker && activeMention ? (
						<FileMentionPicker
							query={activeMention.query}
							selectedIndex={mentionIndex}
							onSelect={applyMention}
							onResultsChange={setMentionResults}
						/>
					) : null}
					<textarea
						ref={textareaRef}
						value={value}
						onChange={(e) => {
							setValue(e.target.value)
							setMentionIndex(0)
						}}
						onClick={() => setMentionIndex(0)}
						onKeyDown={(e) => {
							if (showMentionPicker) {
								if (e.key === "ArrowDown") {
									e.preventDefault()
									setMentionIndex((i) =>
										mentionResults.length > 0
											? (i + 1) % mentionResults.length
											: i + 1,
									)
									return
								}
								if (e.key === "ArrowUp") {
									e.preventDefault()
									setMentionIndex((i) =>
										mentionResults.length > 0
											? (i - 1 + mentionResults.length) %
												mentionResults.length
											: Math.max(0, i - 1),
									)
									return
								}
								if (e.key === "Escape") {
									e.preventDefault()
									setMentionIndex(0)
									return
								}
								if (e.key === "Tab" || e.key === "Enter") {
									if (selectHighlightedMention()) {
										e.preventDefault()
									}
									return
								}
							}

							if (e.key === "Enter" && !e.shiftKey && !(e.metaKey || e.ctrlKey)) {
								e.preventDefault()
								void handleSubmit()
							}

							if (
								e.key === "Enter" &&
								(e.metaKey || e.ctrlKey) &&
								!e.shiftKey
							) {
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
									: "Message the agent… (@ to attach a file)"
						}
						className="w-full resize-none bg-transparent px-3 py-2.5 text-sm text-fg outline-none placeholder:text-muted disabled:opacity-50"
					/>
					<div className="flex items-center justify-end gap-1.5 border-t border-border px-2 py-1.5">
						{promptInFlight ? (
							<button
								type="button"
								onClick={() => void handleCancel()}
								className="inline-flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1 text-xs font-medium text-fg transition hover:bg-white/5"
							>
								<Square className="size-3 fill-current" />
								Stop
							</button>
						) : null}
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
