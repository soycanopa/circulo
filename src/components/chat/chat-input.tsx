import { useAtomValue, useSetAtom } from "jotai"
import { CornerDownLeft, Loader2, Square } from "lucide-react"
import { useRef, useState } from "react"
import { ConfigSelectors } from "@/components/chat/config-selector"
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
	activeSessionIdAtom,
	errorMessageAtom,
	sessionsAtom,
	visiblePromptInFlightAtom,
} from "@/stores/atoms"
import type { ChatMessage } from "@/types/acp"

export function ChatInput() {
	const sessionId = useAtomValue(activeSessionIdAtom)
	const promptInFlight = useAtomValue(visiblePromptInFlightAtom)
	const permission = useAtomValue(activePermissionAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const setError = useSetAtom(errorMessageAtom)
	const [value, setValue] = useState("")
	const [mentionIndex, setMentionIndex] = useState(0)
	const [mentionResults, setMentionResults] = useState<string[]>([])
	const textareaRef = useRef<HTMLTextAreaElement>(null)

	const disabled = !sessionId || promptInFlight || Boolean(permission)

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

		// Optimistic update on the active session only — never overwrite the
		// legacy global atoms (they no longer exist).
		const targetSid = sessionId
		if (targetSid) {
			const now = Date.now()
			const optimistic: ChatMessage[] = [
				{
					id: crypto.randomUUID(),
					role: "user",
					content: trimmed,
					toolCalls: [],
					timestamp: now,
				},
				{
					id: crypto.randomUUID(),
					role: "assistant",
					content: "",
					toolCalls: [],
					timestamp: now,
				},
			]
			setSessions((prev) => {
				const current = prev[targetSid]
				if (!current) return prev
				return {
					...prev,
					[targetSid]: {
						...current,
						messages: [...current.messages, ...optimistic],
						promptInFlight: true,
						status: "generating",
					},
				}
			})
		}
		setValue("")
		setMentionIndex(0)
		setMentionResults([])
		setError(null)

		try {
			await sendPrompt(trimmed, contextPaths)
		} catch (error) {
			setSessions((prev) => {
				if (!targetSid) return prev
				const current = prev[targetSid]
				if (!current) return prev
				return {
					...prev,
					[targetSid]: {
						...current,
						promptInFlight: false,
						status: "idle",
					},
				}
			})
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
		<div className="shrink-0 px-4 py-3">
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
					{/* Model / mode + send — same row, no divider line above */}
					<div className="flex items-center gap-2 px-2.5 pb-2 pt-0.5">
						<div className="min-w-0 flex-1">
							<ConfigSelectors />
						</div>
						<div className="flex shrink-0 items-center gap-1.5">
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
								title="Send"
								aria-label="Send"
								className="inline-flex size-8 shrink-0 items-center justify-center rounded-md bg-white/10 text-fg transition hover:bg-white/15 disabled:opacity-40"
							>
								{promptInFlight ? (
									<Loader2 className="size-4 animate-spin" />
								) : (
									<CornerDownLeft className="size-4" />
								)}
							</button>
						</div>
					</div>
				</form>
			</div>
		</div>
	)
}
