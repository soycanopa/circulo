import { useSetAtom } from "jotai"
import { AtSign, Send } from "lucide-react"
import { useEffect, useMemo, useState } from "react"
import { Button } from "@/components/ui/button"
import { searchFiles, sendPrompt } from "@/lib/tauri"
import { messagesAtom } from "@/stores/atoms"
import { cn } from "@/lib/utils"
import type { MentionChip, SessionStatus } from "@/types/acp"

interface ChatInputProps {
	disabled?: boolean
	sessionStatus: SessionStatus
	onSubmit?: () => void
}

function extractMentionQuery(value: string, caret: number) {
	const beforeCaret = value.slice(0, caret)
	const match = /(?:^|\s)@([^\s@]*)$/.exec(beforeCaret)
	return match ? match[1] : null
}

function extractMentionPaths(value: string): string[] {
	const matches = value.matchAll(/@([^\s@]+)/g)
	return [...matches].map((match) => match[1])
}

export function ChatInput({ disabled, sessionStatus, onSubmit }: ChatInputProps) {
	const setMessages = useSetAtom(messagesAtom)
	const [value, setValue] = useState("")
	const [mentions, setMentions] = useState<MentionChip[]>([])
	const [query, setQuery] = useState<string | null>(null)
	const [suggestions, setSuggestions] = useState<string[]>([])
	const [caret, setCaret] = useState(0)

	const isBusy = sessionStatus === "generating" || sessionStatus === "awaiting_permission"

	useEffect(() => {
		if (query === null) {
			setSuggestions([])
			return
		}

		const timeout = setTimeout(() => {
			searchFiles(query)
				.then(setSuggestions)
				.catch(() => setSuggestions([]))
		}, 120)

		return () => clearTimeout(timeout)
	}, [query])

	const visibleSuggestions = useMemo(() => suggestions.slice(0, 8), [suggestions])

	function updateMentionsFromValue(nextValue: string) {
		const paths = extractMentionPaths(nextValue)
		setMentions(
			paths.map((path) => ({
				path,
				label: path.split("/").pop() ?? path,
			})),
		)
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
		const nextCaret = replaced.length
		setValue(nextValue)
		setCaret(nextCaret)
		updateMentionsFromValue(nextValue)
		setQuery(null)
	}

	async function handleSubmit() {
		const trimmed = value.trim()
		if (!trimmed || disabled || isBusy) return

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
		await sendPrompt(trimmed, contextPaths)
		setValue("")
		setMentions([])
		setQuery(null)
		onSubmit?.()
	}

	return (
		<div className="border-t border-border bg-card/80 p-4">
			{mentions.length > 0 ? (
				<div className="mb-2 flex flex-wrap gap-2">
					{mentions.map((mention) => (
						<span
							key={mention.path}
							className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-1 text-xs"
						>
							<AtSign className="size-3" />
							{mention.label}
						</span>
					))}
				</div>
			) : null}

			<div className="relative">
				{visibleSuggestions.length > 0 && query !== null ? (
					<div className="absolute bottom-full left-0 z-20 mb-2 w-full overflow-hidden rounded-md border border-border bg-popover shadow-lg">
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

				<div className="flex items-end gap-2">
					<textarea
						value={value}
						disabled={disabled || isBusy}
						placeholder="Escribe un mensaje… Usa @ para referenciar archivos"
						className={cn(
							"min-h-24 flex-1 resize-none rounded-lg border border-input bg-muted/30 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-ring/40",
							(disabled || isBusy) && "opacity-60",
						)}
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
					<Button
						disabled={disabled || isBusy || !value.trim()}
						onClick={() => void handleSubmit()}
						aria-label="Enviar mensaje"
					>
						<Send className="size-4" />
					</Button>
				</div>
			</div>
		</div>
	)
}