import { useAtom, useAtomValue, useSetAtom } from "jotai"
import { AtSign, CornerDownLeft, Loader2 } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import { AgentModeSelector } from "@/components/chat/agent-mode-selector"
import { ContextWindowMeter } from "@/components/chat/context-window-meter"
import { ModelSelector } from "@/components/chat/model-selector"
import { SlashCommandPicker } from "@/components/chat/slash-command-picker"
import { ThinkingSelector } from "@/components/chat/thinking-selector"
import { ThreadFolderPicker } from "@/components/chat/thread-folder-picker"
import { useSlashEntries } from "@/hooks/use-slash-entries"
import { CredentialPrompt } from "@/components/credentials/credential-prompt"
import { PermissionPrompt } from "@/components/permissions/permission-prompt"
import { isAgentPlanMode } from "@/lib/agent-mode"
import { setPromptInFlightSync } from "@/lib/prompt-flight"
import { resolveContextWindowDisplay } from "@/lib/context-window"
import { deriveTitleFromMessage } from "@/lib/sessions"
import {
	InputGroup,
	InputGroupAddon,
	InputGroupButton,
	InputGroupTextarea,
} from "@/components/ui/input-group"
import {
	expandSlashPrompt,
	extractSlashQuery,
	filterSlashEntries,
	type SlashEntry,
} from "@/lib/slash-prompt"
import { searchFiles, sendPrompt } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import {
	activeCredentialAtom,
	activePermissionAtom,
	activeSessionIdAtom,
	configOptionsAtom,
	contextWindowAtom,
	messagesAtom,
	NEW_THREAD_PICKER_ID,
	pendingPlanAtom,
	planCommentModeAtom,
	planTurnActiveAtom,
	projectPathAtom,
	promptInFlightAtom,
	sessionsAtom,
	threadFolderPickerSessionIdAtom,
} from "@/stores/atoms"
import type { MentionChip, SessionStatus } from "@/types/acp"

interface ChatInputProps {
	disabled?: boolean
	sessionStatus: SessionStatus
	onOpenProject: (path: string) => Promise<void>
	onOpenProjectForNewThread: (path: string) => Promise<void>
}

function extractMentionQuery(value: string, caret: number) {
	const beforeCaret = value.slice(0, caret)
	const match = /(?:^|\s)@([^\s@]*)$/.exec(beforeCaret)
	return match ? match[1] : null
}

function extractMentionPaths(value: string): string[] {
	return [...value.matchAll(/@([^\s@]+)/g)].map((match) => match[1])
}

export function ChatInput({
	disabled,
	sessionStatus,
	onOpenProject,
	onOpenProjectForNewThread,
}: ChatInputProps) {
	const setMessages = useSetAtom(messagesAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const [activeSessionId] = useAtom(activeSessionIdAtom)
	const [pickerSessionId, setThreadFolderPickerSessionId] = useAtom(
		threadFolderPickerSessionIdAtom,
	)
	const projectPath = useAtomValue(projectPathAtom)
	const configOptions = useAtomValue(configOptionsAtom)
	const contextWindowUsage = useAtomValue(contextWindowAtom)
	const contextWindow = useMemo(
		() => resolveContextWindowDisplay(contextWindowUsage, configOptions),
		[contextWindowUsage, configOptions],
	)
	const setPlanTurnActive = useSetAtom(planTurnActiveAtom)
	const messageCount = useAtomValue(messagesAtom).length
	const isPendingNewThreadFolder = pickerSessionId === NEW_THREAD_PICKER_ID
	const showFolderPicker =
		Boolean(pickerSessionId) &&
		messageCount === 0 &&
		(isPendingNewThreadFolder || pickerSessionId === activeSessionId)
	const pickerProjectPath = isPendingNewThreadFolder ? null : projectPath
	const activeCredential = useAtomValue(activeCredentialAtom)
	const activePermission = useAtomValue(activePermissionAtom)
	const [promptInFlight, setPromptInFlight] = useAtom(promptInFlightAtom)
	const showCredentialPrompt = Boolean(activeCredential)
	const showPermissionPrompt = Boolean(activePermission) && !showCredentialPrompt
	const [planCommentMode, setPlanCommentMode] = useAtom(planCommentModeAtom)
	const setPendingPlan = useSetAtom(pendingPlanAtom)
	const textareaRef = useRef<HTMLTextAreaElement>(null)
	const [value, setValue] = useState("")
	const [mentions, setMentions] = useState<MentionChip[]>([])
	const [mentionQuery, setMentionQuery] = useState<string | null>(null)
	const [slashQuery, setSlashQuery] = useState<string | null>(null)
	const [suggestions, setSuggestions] = useState<string[]>([])
	const [slashSelectedIndex, setSlashSelectedIndex] = useState(0)
	const [caret, setCaret] = useState(0)
	const { entries: slashEntries, commands, skills, mcpServers } =
		useSlashEntries(projectPath)

	const isAwaitingInput =
		sessionStatus === "awaiting_permission" || sessionStatus === "awaiting_credential"
	const isSubmitting = promptInFlight

	useEffect(() => {
		if (!planCommentMode) return
		textareaRef.current?.focus()
	}, [planCommentMode])

	useEffect(() => {
		if (mentionQuery === null) {
			setSuggestions([])
			return
		}
		const timeout = setTimeout(() => {
			searchFiles(mentionQuery).then(setSuggestions).catch(() => setSuggestions([]))
		}, 120)
		return () => clearTimeout(timeout)
	}, [mentionQuery])

	const visibleSuggestions = useMemo(() => suggestions.slice(0, 8), [suggestions])
	const visibleSlashEntries = useMemo(
		() => (slashQuery === null ? [] : filterSlashEntries(slashEntries, slashQuery)),
		[slashEntries, slashQuery],
	)

	useEffect(() => {
		setSlashSelectedIndex(0)
	}, [slashQuery, visibleSlashEntries.length])

	function updateMentionsFromValue(nextValue: string) {
		const paths = extractMentionPaths(nextValue)
		setMentions(paths.map((path) => ({ path, label: path.split("/").pop() ?? path })))
	}

	function handleChange(nextValue: string, nextCaret: number) {
		setValue(nextValue)
		setCaret(nextCaret)
		updateMentionsFromValue(nextValue)
		const nextMentionQuery = extractMentionQuery(nextValue, nextCaret)
		const nextSlashQuery = extractSlashQuery(nextValue, nextCaret)
		if (nextMentionQuery !== null) {
			setMentionQuery(nextMentionQuery)
			setSlashQuery(null)
			return
		}
		if (nextSlashQuery !== null) {
			setSlashQuery(nextSlashQuery)
			setMentionQuery(null)
			return
		}
		setMentionQuery(null)
		setSlashQuery(null)
	}

	function insertMention(path: string) {
		const beforeCaret = value.slice(0, caret)
		const afterCaret = value.slice(caret)
		const replaced = beforeCaret.replace(/@([^\s@]*)$/, `@${path} `)
		const nextValue = `${replaced}${afterCaret}`
		setValue(nextValue)
		setCaret(replaced.length)
		updateMentionsFromValue(nextValue)
		setMentionQuery(null)
	}

	function insertSlash(entry: SlashEntry) {
		const beforeCaret = value.slice(0, caret)
		const afterCaret = value.slice(caret)
		const replaced = beforeCaret.replace(/\/([^\s/]*)$/, `/${entry.name} `)
		const nextValue = `${replaced}${afterCaret}`
		setValue(nextValue)
		setCaret(replaced.length)
		updateMentionsFromValue(nextValue)
		setSlashQuery(null)
		setMentionQuery(null)
		textareaRef.current?.focus()
	}

	async function handleSubmit(event?: React.FormEvent) {
		event?.preventDefault()
		const trimmed = value.trim()
		if (!trimmed || disabled || isAwaitingInput || isSubmitting || isPendingNewThreadFolder) {
			return
		}

		const expanded = planCommentMode
			? `Comentarios sobre el plan:\n\n${trimmed}`
			: expandSlashPrompt(trimmed, commands, skills, mcpServers)
		const promptText = expanded
		const contextPaths = mentions.map((mention) => mention.path)
		setMessages((current) => [
			...current,
			{
				id: crypto.randomUUID(),
				role: "user",
				content: trimmed,
				toolCalls: [],
				timestamp: Date.now(),
			},
		])
		if (planCommentMode) setPlanCommentMode(false)
		setPendingPlan(null)
		setPlanTurnActive(isAgentPlanMode(configOptions))

		if (activeSessionId) {
			setSessions((current) =>
				current.map((session) => {
					if (session.sessionId !== activeSessionId) return session
					if (session.title?.trim()) return session
					return { ...session, title: deriveTitleFromMessage(trimmed) }
				}),
			)
		}
		setThreadFolderPickerSessionId(null)
		setPromptInFlightSync(true)
		setPromptInFlight(true)
		setValue("")
		setMentions([])
		setMentionQuery(null)
		setSlashQuery(null)
		try {
			await sendPrompt(promptText, contextPaths)
		} catch {
			setPromptInFlightSync(false)
			setPromptInFlight(false)
		}
	}

	const inputDisabled = disabled || isAwaitingInput || isPendingNewThreadFolder

	return (
		<div className="relative z-10 shrink-0 overflow-visible px-4 pb-4 pt-2">
			<div className="relative mx-auto max-w-3xl">
				{visibleSuggestions.length > 0 && mentionQuery !== null ? (
					<div className="absolute bottom-full left-0 z-20 mb-2 w-full overflow-hidden rounded-lg border border-popover-border bg-popover shadow-lg">
						{visibleSuggestions.map((path) => (
							<button
								key={path}
								type="button"
								className="block w-full px-3 py-2 text-left text-sm hover:bg-accent"
								onClick={() => insertMention(path)}
							>
								{path}
							</button>
						))}
					</div>
				) : null}
				{slashQuery !== null ? (
					<SlashCommandPicker
						query={slashQuery}
						entries={slashEntries}
						selectedIndex={slashSelectedIndex}
						onSelect={insertSlash}
						onHover={setSlashSelectedIndex}
					/>
				) : null}

				<form onSubmit={(e) => void handleSubmit(e)}>
					<InputGroup>
						{showFolderPicker ? (
							<div data-slot="thread-selectors">
								<ThreadFolderPicker
									projectPath={pickerProjectPath}
									onOpenProject={
										isPendingNewThreadFolder ? onOpenProjectForNewThread : onOpenProject
									}
									onClose={() => setThreadFolderPickerSessionId(null)}
								/>
							</div>
						) : null}
						{showCredentialPrompt ? (
							<CredentialPrompt />
						) : showPermissionPrompt ? (
							<PermissionPrompt />
						) : (
							<>
								{mentions.length > 0 ? (
									<div className="flex flex-wrap gap-1.5 px-3 pt-2">
										{mentions.map((mention) => (
											<span
												key={mention.path}
												className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs"
											>
												<AtSign className="size-3" />
												{mention.label}
											</span>
										))}
									</div>
								) : null}

								<InputGroupTextarea
									ref={textareaRef}
									value={value}
									disabled={inputDisabled}
									placeholder={
										planCommentMode
											? "Comenta el plan: qué cambiar, qué priorizar, qué rechazar…"
											: "Escribe un mensaje… @ archivos · / commands, skills y MCP"
									}
									className={cn(inputDisabled && "opacity-60")}
									onChange={(event) =>
										handleChange(event.target.value, event.target.selectionStart ?? 0)
									}
									onSelect={(event) =>
										setCaret((event.target as HTMLTextAreaElement).selectionStart ?? 0)
									}
									onKeyDown={(event) => {
										if (slashQuery !== null && visibleSlashEntries.length > 0) {
											if (event.key === "ArrowDown") {
												event.preventDefault()
												setSlashSelectedIndex(
													(current) => (current + 1) % visibleSlashEntries.length,
												)
												return
											}
											if (event.key === "ArrowUp") {
												event.preventDefault()
												setSlashSelectedIndex(
													(current) =>
														(current - 1 + visibleSlashEntries.length) %
														visibleSlashEntries.length,
												)
												return
											}
											if (event.key === "Enter" && !event.shiftKey) {
												event.preventDefault()
												const entry = visibleSlashEntries[slashSelectedIndex]
												if (entry) insertSlash(entry)
												return
											}
											if (event.key === "Escape") {
												event.preventDefault()
												setSlashQuery(null)
												return
											}
										}
										if (event.key === "Enter" && !event.shiftKey) {
											event.preventDefault()
											void handleSubmit()
										}
									}}
								/>

								<InputGroupAddon className="justify-between overflow-visible">
									<div className="flex min-w-0 flex-wrap items-center gap-1 overflow-visible">
										<AgentModeSelector />
										<ModelSelector />
										<ThinkingSelector />
										<ContextWindowMeter usage={contextWindow} />
									</div>
									<div className="flex items-center gap-2">
										<InputGroupButton
											type="submit"
											variant="default"
											disabled={inputDisabled || isSubmitting || !value.trim()}
											aria-label="Enviar mensaje"
										>
											{isSubmitting ? (
												<Loader2 className="size-4 animate-spin text-[#3B5EF9]" />
											) : (
												<CornerDownLeft className="size-4" />
											)}
										</InputGroupButton>
									</div>
								</InputGroupAddon>
							</>
						)}
					</InputGroup>
				</form>
			</div>
		</div>
	)
}
