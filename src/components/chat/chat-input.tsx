import { useAtom, useAtomValue, useSetAtom } from "jotai"
import { AtSign, CornerDownLeft, Loader2 } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import { AgentModeSelector } from "@/components/chat/agent-mode-selector"
import { ContextWindowMeter } from "@/components/chat/context-window-meter"
import { FileMentionPicker } from "@/components/chat/file-mention-picker"
import { ModelSelector } from "@/components/chat/model-selector"
import {
	SlashCommandPicker,
	slashEntryKey,
} from "@/components/chat/slash-command-picker"
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

function moveSelection<T>(
	items: T[],
	currentValue: string,
	direction: 1 | -1,
	getValue: (item: T) => string,
) {
	if (items.length === 0) return ""
	const currentIndex = items.findIndex((item) => getValue(item) === currentValue)
	const nextIndex =
		currentIndex < 0
			? 0
			: (currentIndex + direction + items.length) % items.length
	return getValue(items[nextIndex])
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
	const [isSearchingFiles, setIsSearchingFiles] = useState(false)
	const [mentionSelectedValue, setMentionSelectedValue] = useState("")
	const [slashSelectedValue, setSlashSelectedValue] = useState("")
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
			setIsSearchingFiles(false)
			return
		}
		setIsSearchingFiles(true)
		const timeout = setTimeout(() => {
			searchFiles(mentionQuery)
				.then((results) => {
					setSuggestions(results)
					setMentionSelectedValue(results[0] ?? "")
				})
				.catch(() => {
					setSuggestions([])
					setMentionSelectedValue("")
				})
				.finally(() => setIsSearchingFiles(false))
		}, 120)
		return () => clearTimeout(timeout)
	}, [mentionQuery])

	const visibleSlashEntries = useMemo(
		() => (slashQuery === null ? [] : filterSlashEntries(slashEntries, slashQuery)),
		[slashEntries, slashQuery],
	)

	useEffect(() => {
		setSlashSelectedValue(
			visibleSlashEntries[0] ? slashEntryKey(visibleSlashEntries[0]) : "",
		)
	}, [slashQuery, visibleSlashEntries])

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

	function updateActivePickerQuery(nextQuery: string, trigger: "@" | "/") {
		const beforeCaret = value.slice(0, caret)
		const afterCaret = value.slice(caret)
		const pattern = trigger === "@" ? /@([^\s@]*)$/ : /\/([^\s/]*)$/
		const replaced = beforeCaret.replace(pattern, `${trigger}${nextQuery}`)
		const nextValue = `${replaced}${afterCaret}`
		const nextCaret = replaced.length
		handleChange(nextValue, nextCaret)
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
		textareaRef.current?.focus()
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

	function handleComposerPickerKeyDown(event: React.KeyboardEvent<HTMLTextAreaElement>) {
		if (slashQuery !== null) {
			if (event.key === "ArrowDown") {
				event.preventDefault()
				setSlashSelectedValue((current) =>
					moveSelection(visibleSlashEntries, current, 1, slashEntryKey),
				)
				return true
			}
			if (event.key === "ArrowUp") {
				event.preventDefault()
				setSlashSelectedValue((current) =>
					moveSelection(visibleSlashEntries, current, -1, slashEntryKey),
				)
				return true
			}
			if (event.key === "Enter" && !event.shiftKey) {
				event.preventDefault()
				const entry = visibleSlashEntries.find(
					(item) => slashEntryKey(item) === slashSelectedValue,
				)
				if (entry) insertSlash(entry)
				return true
			}
			if (event.key === "Escape") {
				event.preventDefault()
				setSlashQuery(null)
				return true
			}
			return false
		}

		if (mentionQuery !== null) {
			if (event.key === "ArrowDown") {
				event.preventDefault()
				setMentionSelectedValue((current) =>
					moveSelection(suggestions, current, 1, (path) => path),
				)
				return true
			}
			if (event.key === "ArrowUp") {
				event.preventDefault()
				setMentionSelectedValue((current) =>
					moveSelection(suggestions, current, -1, (path) => path),
				)
				return true
			}
			if (event.key === "Enter" && !event.shiftKey && mentionSelectedValue) {
				event.preventDefault()
				insertMention(mentionSelectedValue)
				return true
			}
			if (event.key === "Escape") {
				event.preventDefault()
				setMentionQuery(null)
				return true
			}
			return false
		}

		return false
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
				{mentionQuery !== null ? (
					<FileMentionPicker
						query={mentionQuery}
						files={suggestions}
						isLoading={isSearchingFiles}
						hasProject={Boolean(projectPath)}
						selectedValue={mentionSelectedValue}
						onSelect={insertMention}
						onValueChange={setMentionSelectedValue}
						onQueryChange={(nextQuery) => updateActivePickerQuery(nextQuery, "@")}
					/>
				) : null}
				{slashQuery !== null ? (
					<SlashCommandPicker
						query={slashQuery}
						entries={slashEntries}
						selectedValue={slashSelectedValue}
						onSelect={insertSlash}
						onValueChange={setSlashSelectedValue}
						onQueryChange={(nextQuery) => updateActivePickerQuery(nextQuery, "/")}
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
										if (handleComposerPickerKeyDown(event)) return
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