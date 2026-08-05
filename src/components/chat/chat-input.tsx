import { useAtomValue, useSetAtom } from "jotai"
import { getDefaultStore } from "jotai"
import { CornerDownLeft, Loader2, Square } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import { ConfigSelectors } from "@/components/chat/config-selector"
import { AgentSelector } from "@/components/chat/agent-selector"
import { BranchSelector } from "@/components/chat/branch-selector"
import { FileMentionPicker } from "@/components/chat/file-mention-picker"
import { SlashMenu } from "@/components/chat/slash-menu"
import { PermissionPrompt } from "@/components/permissions/permission-prompt"
import { reconcileSessionFromProjectStatus } from "@/hooks/session-reconcile"
import { useAutoApprove } from "@/hooks/use-auto-approve"
import {
	extractMentionPaths,
	getActiveMention,
	insertMention,
} from "@/lib/mention-parser"
import {
	mergeSlashCommands,
	type SlashCommand,
} from "@/lib/slash-commands"
import {
	filterSlashCommands,
	getActiveSlash,
} from "@/lib/slash-parser"
import { cancelPrompt, createSession, sendPrompt } from "@/lib/tauri"
import {
	activePermissionAtom,
	activeSessionIdAtom,
	agentConnectedAtom,
	appSettingsAtom,
	composerInsertRequestAtom,
	draftBySessionAtom,
	errorMessageAtom,
	projectPathAtom,
	sessionsAtom,
	setDraftAtom,
	visiblePromptInFlightAtom,
} from "@/stores/atoms"
import type { ChatMessage } from "@/types/acp"

interface ChatInputProps {
	enabledAgentIds?: string[]
	preferredAgentId?: string | null
	onAgentChange?: (agentId: string) => void | Promise<void>
	onNewChat?: () => void
}

export function ChatInput({
	enabledAgentIds = [],
	preferredAgentId,
	onAgentChange,
	onNewChat,
}: ChatInputProps) {
	useAutoApprove()
	const sessionId = useAtomValue(activeSessionIdAtom)
	const agentConnected = useAtomValue(agentConnectedAtom)
	const projectPath = useAtomValue(projectPathAtom)
	const appSettings = useAtomValue(appSettingsAtom)
	const promptInFlight = useAtomValue(visiblePromptInFlightAtom)
	const permission = useAtomValue(activePermissionAtom)
	const drafts = useAtomValue(draftBySessionAtom)
	const setDraft = useSetAtom(setDraftAtom)
	const composerInsert = useAtomValue(composerInsertRequestAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const setError = useSetAtom(errorMessageAtom)
	const [value, setValue] = useState("")
	const [startingSession, setStartingSession] = useState(false)
	const [mentionIndex, setMentionIndex] = useState(0)
	const [mentionResults, setMentionResults] = useState<string[]>([])
	const [slashIndex, setSlashIndex] = useState(0)
	const textareaRef = useRef<HTMLTextAreaElement>(null)
	const draftSessionRef = useRef<string | null>(null)

	// Restore the per-session draft when the visible session changes.
	useEffect(() => {
		const current = sessionId ?? ""
		if (draftSessionRef.current === current) return
		draftSessionRef.current = current
		setValue(current ? (drafts[current] ?? "") : "")
	}, [sessionId, drafts])

	// Append feedback/diff comments requested from other panels (e.g. DiffPanel).
	useEffect(() => {
		if (!composerInsert) return
		setValue((prev) =>
			prev.trim() ? `${prev.trim()}\n\n${composerInsert.text}` : composerInsert.text,
		)
	}, [composerInsert])

	const disabled =
		!agentConnected || promptInFlight || Boolean(permission) || startingSession

	const cursor = textareaRef.current?.selectionStart ?? value.length
	const activeMention = getActiveMention(value, cursor)
	const showMentionPicker = Boolean(activeMention && projectPath && !disabled)

	const activeSlash = getActiveSlash(value, cursor)
	const allSlashCommands = useMemo(
		() =>
			mergeSlashCommands(appSettings?.customSlashCommands ?? []),
		[appSettings?.customSlashCommands],
	)
	const slashResults = activeSlash
		? filterSlashCommands(activeSlash.query, allSlashCommands)
		: []
	const showSlashMenu = Boolean(activeSlash && !disabled)

	function updateValue(next: string, nextCursor: number) {
		setValue(next)
		if (sessionId) setDraft(sessionId, next)
		requestAnimationFrame(() => {
			const el = textareaRef.current
			if (!el) return
			el.focus()
			el.setSelectionRange(nextCursor, nextCursor)
		})
	}

	function handleValueChange(next: string) {
		setValue(next)
		setMentionIndex(0)
		setSlashIndex(0)
		if (sessionId) setDraft(sessionId, next)
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

	function selectHighlightedSlash(): boolean {
		if (!showSlashMenu || slashResults.length === 0) return false
		const command =
			slashResults[slashIndex % slashResults.length] ?? slashResults[0]
		if (!command) return false
		applySlash(command)
		return true
	}

	async function ensureSessionId(): Promise<string | null> {
		const current = getDefaultStore().get(activeSessionIdAtom)
		if (current) return current

		setStartingSession(true)
		try {
			const status = await createSession()
			reconcileSessionFromProjectStatus(getDefaultStore(), status)
			return (
				status.sessionId ?? getDefaultStore().get(activeSessionIdAtom) ?? null
			)
		} finally {
			setStartingSession(false)
		}
	}

	async function submitText(text: string) {
		const trimmed = text.trim()
		if (!trimmed || disabled) return

		const contextPaths = extractMentionPaths(trimmed)

		let targetSid = sessionId
		if (!targetSid) {
			try {
				targetSid = await ensureSessionId()
			} catch (error) {
				setError(
					error instanceof Error ? error.message : "Failed to start session",
				)
				return
			}
		}
		if (!targetSid) {
			setError("No active session — wait for the agent to finish starting")
			return
		}

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
			if (!current) {
				return {
					...prev,
					[targetSid]: {
						messages: optimistic,
						streaming: "",
						promptInFlight: true,
						status: "generating",
						configOptions: [],
						contextUsage: null,
					},
				}
			}
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
		setValue("")
		setMentionIndex(0)
		setMentionResults([])
		setSlashIndex(0)
		setError(null)
		setDraft(targetSid, "")

		try {
			await sendPrompt(trimmed, contextPaths)
		} catch (error) {
			setSessions((prev) => {
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

	async function handleSubmit(event?: React.FormEvent) {
		event?.preventDefault()
		await submitText(value)
	}

	function applySlash(command: SlashCommand) {
		if (!activeSlash) return
		if (command.action === "new-chat") {
			setValue("")
			if (sessionId) setDraft(sessionId, "")
			setSlashIndex(0)
			onNewChat?.()
			return
		}
		// Send the command on its own; drop anything already typed after the token.
		void submitText(command.prompt ?? command.label)
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
					{showSlashMenu && activeSlash ? (
						<SlashMenu
							results={slashResults}
							selectedIndex={slashIndex}
							onSelect={applySlash}
						/>
					) : null}
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
						onChange={(e) => handleValueChange(e.target.value)}
						onClick={() => {
							setMentionIndex(0)
							setSlashIndex(0)
						}}
						onKeyDown={(e) => {
							if (showSlashMenu) {
								if (e.key === "ArrowDown") {
									e.preventDefault()
									setSlashIndex((i) =>
										slashResults.length > 0
											? (i + 1) % slashResults.length
											: i + 1,
									)
									return
								}
								if (e.key === "ArrowUp") {
									e.preventDefault()
									setSlashIndex((i) =>
										slashResults.length > 0
											? (i - 1 + slashResults.length) %
												slashResults.length
											: Math.max(0, i - 1),
									)
									return
								}
								if (e.key === "Escape") {
									e.preventDefault()
									setSlashIndex(0)
									return
								}
								if (e.key === "Tab" || e.key === "Enter") {
									if (selectHighlightedSlash()) {
										e.preventDefault()
									}
									return
								}
							}

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
							startingSession
								? "Starting session…"
								: permission
									? "Respond to the permission request…"
									: "Message the agent… (@ to attach a file)"
						}
						className="w-full resize-none bg-transparent px-3 py-2.5 text-sm text-fg outline-none placeholder:text-muted disabled:opacity-50"
					/>
					{/* Model / mode + send — same row, no divider line above */}
					<div className="flex items-center gap-2 px-2.5 pb-2 pt-0.5">
						<div className="flex min-w-0 flex-1 items-center gap-1">
							{onAgentChange && enabledAgentIds.length > 0 ? (
								<AgentSelector
									enabledAgentIds={enabledAgentIds}
									preferredAgentId={preferredAgentId}
									onAgentChange={onAgentChange}
								/>
							) : null}
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
								{promptInFlight || startingSession ? (
									<Loader2 className="size-4 animate-spin" />
								) : (
									<CornerDownLeft className="size-4" />
								)}
							</button>
						</div>
					</div>
				</form>

				{/* Branch selector hangs just below the composer's bottom border. */}
				<div className="mt-1.5 flex justify-start px-0.5">
					<BranchSelector projectPath={projectPath} />
				</div>
			</div>
		</div>
	)
}
