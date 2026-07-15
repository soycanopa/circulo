import { useAtom, useAtomValue, useSetAtom } from "jotai"
import { AtSign, CornerDownLeft, Loader2 } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import { AgentModeSelector } from "@/components/chat/agent-mode-selector"
import { ModelSelector } from "@/components/chat/model-selector"
import { ThinkingSelector } from "@/components/chat/thinking-selector"
import { ThreadFolderPicker } from "@/components/chat/thread-folder-picker"
import { setPromptInFlightSync } from "@/lib/prompt-flight"
import { deriveTitleFromMessage } from "@/lib/sessions"
import {
	InputGroup,
	InputGroupAddon,
	InputGroupButton,
	InputGroupTextarea,
} from "@/components/ui/input-group"
import { searchFiles, sendPrompt } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import {
	activeSessionIdAtom,
	messagesAtom,
	NEW_THREAD_PICKER_ID,
	pendingPlanAtom,
	planCommentModeAtom,
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
	const messageCount = useAtomValue(messagesAtom).length
	const isPendingNewThreadFolder = pickerSessionId === NEW_THREAD_PICKER_ID
	const showFolderPicker =
		Boolean(pickerSessionId) &&
		messageCount === 0 &&
		(isPendingNewThreadFolder || pickerSessionId === activeSessionId)
	const pickerProjectPath = isPendingNewThreadFolder ? null : projectPath
	const [promptInFlight, setPromptInFlight] = useAtom(promptInFlightAtom)
	const [planCommentMode, setPlanCommentMode] = useAtom(planCommentModeAtom)
	const setPendingPlan = useSetAtom(pendingPlanAtom)
	const textareaRef = useRef<HTMLTextAreaElement>(null)
	const [value, setValue] = useState("")
	const [mentions, setMentions] = useState<MentionChip[]>([])
	const [query, setQuery] = useState<string | null>(null)
	const [suggestions, setSuggestions] = useState<string[]>([])
	const [caret, setCaret] = useState(0)

	const isAwaitingPermission = sessionStatus === "awaiting_permission"
	const isSubmitting = promptInFlight

	useEffect(() => {
		if (!planCommentMode) return
		textareaRef.current?.focus()
	}, [planCommentMode])

	useEffect(() => {
		if (query === null) {
			setSuggestions([])
			return
		}
		const timeout = setTimeout(() => {
			searchFiles(query).then(setSuggestions).catch(() => setSuggestions([]))
		}, 120)
		return () => clearTimeout(timeout)
	}, [query])

	const visibleSuggestions = useMemo(() => suggestions.slice(0, 8), [suggestions])

	function updateMentionsFromValue(nextValue: string) {
		const paths = extractMentionPaths(nextValue)
		setMentions(paths.map((path) => ({ path, label: path.split("/").pop() ?? path })))
	}

	function handleChange(nextValue: string, nextCaret: number) {
		setValue(nextValue)
		setCaret(nextCaret)
		updateMentionsFromValue(nextValue)
		setQuery(extractMentionQuery(nextValue, nextCaret))
	}

	function insertMention(path: string) {
		const beforeCaret = value.slice(0, caret)
		const afterCaret = value.slice(caret)
		const replaced = beforeCaret.replace(/@([^\s@]*)$/, `@${path} `)
		const nextValue = `${replaced}${afterCaret}`
		setValue(nextValue)
		setCaret(replaced.length)
		updateMentionsFromValue(nextValue)
		setQuery(null)
	}

	async function handleSubmit(event?: React.FormEvent) {
		event?.preventDefault()
		const trimmed = value.trim()
		if (!trimmed || disabled || isAwaitingPermission || isSubmitting || isPendingNewThreadFolder) {
			return
		}

		const promptText = planCommentMode
			? `Comentarios sobre el plan:\n\n${trimmed}`
			: trimmed
		const contextPaths = mentions.map((mention) => mention.path)
		setMessages((current) => [
			...current,
			{
				id: crypto.randomUUID(),
				role: "user",
				content: promptText,
				toolCalls: [],
				timestamp: Date.now(),
			},
		])
		if (planCommentMode) setPlanCommentMode(false)
		setPendingPlan(null)

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
		setQuery(null)
		try {
			await sendPrompt(promptText, contextPaths)
		} catch {
			setPromptInFlightSync(false)
			setPromptInFlight(false)
		}
	}

	const inputDisabled = disabled || isAwaitingPermission || isPendingNewThreadFolder

	return (
		<div className="shrink-0 px-4 pb-4 pt-2">
			<div className="relative mx-auto max-w-3xl">
				{visibleSuggestions.length > 0 && query !== null ? (
					<div className="absolute bottom-full left-0 z-20 mb-2 w-full overflow-hidden rounded-lg border border-border bg-popover shadow-lg">
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
									: "Escribe un mensaje… Usa @ para referenciar archivos"
							}
							className={cn(inputDisabled && "opacity-60")}
							onChange={(event) =>
								handleChange(event.target.value, event.target.selectionStart ?? 0)
							}
							onSelect={(event) =>
								setCaret((event.target as HTMLTextAreaElement).selectionStart ?? 0)
							}
							onKeyDown={(event) => {
								if (event.key === "Enter" && !event.shiftKey) {
									event.preventDefault()
									void handleSubmit()
								}
							}}
						/>

						<InputGroupAddon className="justify-between">
							<div className="flex min-w-0 flex-wrap items-center gap-1">
								<AgentModeSelector />
								<ThinkingSelector />
								<ModelSelector />
							</div>
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
						</InputGroupAddon>
					</InputGroup>
				</form>
			</div>
		</div>
	)
}
