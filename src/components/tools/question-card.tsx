import { useAtomValue, useSetAtom } from "jotai"
import { HelpCircle, Loader2, Send } from "lucide-react"
import { useMemo, useState } from "react"
import { sendPrompt } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import {
	activeSessionIdAtom,
	errorMessageAtom,
	sessionsAtom,
} from "@/stores/atoms"
import type { ChatMessage, ToolCall } from "@/types/acp"

interface QuestionOption {
	value: string
	label: string
}

function asRecord(value: unknown): Record<string, unknown> | null {
	return value && typeof value === "object" && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: null
}

function normalizeOptions(value: unknown): QuestionOption[] {
	if (!Array.isArray(value)) return []
	return value.flatMap((item) => {
		if (typeof item === "string") return [{ value: item, label: item }]
		const record = asRecord(item)
		if (!record) return []
		const label = String(record.label ?? record.name ?? record.value ?? "")
		const val = String(record.value ?? label)
		return label ? [{ value: val, label }] : []
	})
}

/** Render a question tool call (radio/checkbox/text) and send the answer. */
export function QuestionCard({ tool }: { tool: ToolCall }) {
	const activeSessionId = useAtomValue(activeSessionIdAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const setError = useSetAtom(errorMessageAtom)
	const [sending, setSending] = useState(false)
	const [answered, setAnswered] = useState(false)
	const [text, setText] = useState("")
	const [selection, setSelection] = useState<Set<string>>(new Set())

	const raw = useMemo(() => asRecord(tool.rawInput), [tool.rawInput])
	const question =
		String(
			raw?.question ??
				raw?.prompt ??
				raw?.message ??
				raw?.text ??
				"",
		) || tool.title
	const type = String(raw?.type ?? "text")
	const multiple = type === "checkbox" || type === "select"
	const options = useMemo(() => normalizeOptions(raw?.options), [raw])

	async function submitAnswer(answer: string) {
		const trimmed = answer.trim()
		if (!trimmed || sending) return
		setSending(true)
		const now = Date.now()
		const optimistic: ChatMessage[] = [
			{
				id: crypto.randomUUID(),
				role: "user",
				content: trimmed,
				toolCalls: [],
				timestamp: now,
			},
		]
		setSessions((prev) => {
			const targetSid = activeSessionId
			if (!targetSid) return prev
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
		setError(null)
		try {
			await sendPrompt(trimmed, [])
			setAnswered(true)
		} catch (error) {
			setError(
				error instanceof Error ? error.message : "Failed to send answer",
			)
		} finally {
			setSending(false)
		}
	}

	function toggleOption(value: string) {
		setSelection((prev) => {
			const next = new Set(prev)
			if (next.has(value)) next.delete(value)
			else next.add(value)
			return next
		})
	}

	function buildAnswer(): string {
		if (options.length > 0) {
			const picked = options
				.filter((option) => selection.has(option.value))
				.map((option) => option.label)
			if (picked.length > 0) {
				return multiple ? picked.join(", ") : picked[0] ?? ""
			}
		}
		return text
	}

	if (answered) {
		return (
			<div className="overflow-hidden rounded-md border border-border bg-surface/80">
				<div className="flex items-center gap-2 px-2.5 py-1.5 text-xs">
					<HelpCircle className="size-3.5 text-muted" />
					<span className="min-w-0 flex-1 truncate font-medium text-fg/90">
						{question}
					</span>
					<span className="shrink-0 rounded bg-emerald-500/15 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-emerald-300">
						answered
					</span>
				</div>
			</div>
		)
	}

	return (
		<div className="overflow-hidden rounded-md border border-indigo-500/25 bg-surface/80">
			<div className="flex items-center gap-2 border-b border-border px-2.5 py-1.5 text-xs">
				<HelpCircle className="size-3.5 shrink-0 text-indigo-300" />
				<span className="min-w-0 flex-1 truncate font-medium text-fg/90">
					{question}
				</span>
				<span className="shrink-0 rounded bg-indigo-500/15 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-indigo-300">
					question
				</span>
			</div>
			<div className="space-y-2 px-2.5 py-2">
				{options.length > 0 ? (
					<div className="flex flex-col gap-1">
						{options.map((option) => {
							const selected = selection.has(option.value)
							return (
								<button
									key={option.value}
									type="button"
									onClick={() =>
										multiple
											? toggleOption(option.value)
											: setSelection(new Set([option.value]))
									}
									className={cn(
										"flex items-center gap-2 rounded border px-2 py-1.5 text-left text-xs transition",
										selected
											? "border-indigo-400/40 bg-indigo-500/10 text-fg"
											: "border-border text-fg/80 hover:bg-white/5",
									)}
								>
									<span
										className={cn(
											"flex size-3.5 shrink-0 items-center justify-center rounded-full border",
											selected && "border-indigo-300 bg-indigo-400",
										)}
									>
										{selected && multiple ? (
											<span className="size-1.5 rounded-full bg-indigo-100" />
										) : null}
									</span>
									<span className="min-w-0 flex-1">{option.label}</span>
								</button>
							)
						})}
					</div>
				) : (
					<input
						value={text}
						onChange={(event) => setText(event.target.value)}
						onKeyDown={(event) => {
							if (event.key === "Enter" && !event.shiftKey) {
								event.preventDefault()
								void submitAnswer(text)
							}
						}}
						placeholder="Type your answer…"
						className="w-full rounded border border-border bg-black/20 px-2 py-1.5 text-xs text-fg outline-none placeholder:text-muted"
					/>
				)}
				<button
					type="button"
					disabled={sending || !buildAnswer()}
					onClick={() => void submitAnswer(buildAnswer())}
					className="inline-flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1 text-[11px] text-fg transition hover:bg-white/5 disabled:opacity-40"
				>
					{sending ? (
						<Loader2 className="size-3 animate-spin" />
					) : (
						<Send className="size-3" />
					)}
					Send answer
				</button>
			</div>
		</div>
	)
}
