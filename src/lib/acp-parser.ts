import type { ChatMessage, ToolCall } from "@/types/acp"

function asRecord(value: unknown): Record<string, unknown> | null {
	return value && typeof value === "object" && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: null
}

function asString(value: unknown): string | undefined {
	return typeof value === "string" ? value : undefined
}

/**
 * Extract text from ACP ContentBlock shapes (and loose OpenCode variants).
 * Spec: { type: "text", text: "..." }
 */
export function extractTextFromContent(content: unknown): string {
	if (!content) return ""
	if (typeof content === "string") return content
	if (Array.isArray(content)) {
		return content.map((item) => extractTextFromContent(item)).join("")
	}
	const record = asRecord(content)
	if (!record) return ""
	if (typeof record.text === "string") return record.text
	if (record.type === "content") return extractTextFromContent(record.content)
	if (record.content !== undefined) return extractTextFromContent(record.content)
	return ""
}

function extractToolContent(content: unknown): string {
	if (!Array.isArray(content)) return extractTextFromContent(content)
	let text = ""
	for (const item of content) {
		const record = asRecord(item)
		if (!record) {
			text += extractTextFromContent(item)
			continue
		}
		if (record.type === "content") text += extractTextFromContent(record.content)
		else if (record.type === "diff") {
			const path = asString(record.path) ?? "file"
			text += `[diff ${path}]\n`
		} else text += extractTextFromContent(item)
	}
	return text
}

/** Merges delta or cumulative stream chunks without duplicating text. */
function mergeStreamText(current: string, chunk: string): string {
	if (!chunk) return current
	if (!current) return chunk
	if (chunk === current) return current
	if (chunk.startsWith(current)) return chunk
	if (current.startsWith(chunk)) return current
	if (current.endsWith(chunk)) return current
	return `${current}${chunk}`
}

function ensureAssistantMessage(messages: ChatMessage[]): ChatMessage {
	const last = messages[messages.length - 1]
	if (last?.role === "assistant") return last
	const created: ChatMessage = {
		id: crypto.randomUUID(),
		role: "assistant",
		content: "",
		toolCalls: [],
		timestamp: Date.now(),
	}
	messages.push(created)
	return created
}

function mapToolFromUpdate(update: Record<string, unknown>): ToolCall {
	const id =
		asString(update.toolCallId) ??
		asString(update.id) ??
		crypto.randomUUID()
	const title =
		asString(update.title) ?? asString(update.kind) ?? "tool"
	const status = asString(update.status) ?? "pending"
	const kind = asString(update.kind)
	const content = extractToolContent(update.content)

	return {
		id,
		title,
		kind,
		status,
		content,
		rawInput: update.rawInput ?? update.input,
		rawOutput: update.rawOutput ?? update.output,
	}
}

/**
 * Apply an ACP `session/update` notification to the chat transcript.
 * Per ACP prompt-turn docs, agent_message_chunk streams text before the
 * session/prompt RPC resolves — the UI must paint these immediately.
 */
export function applySessionUpdate(
	messages: ChatMessage[],
	streamingText: string,
	payload: unknown,
): { messages: ChatMessage[]; streamingText: string; didStream: boolean } {
	const root = asRecord(payload)
	if (!root) return { messages, streamingText, didStream: false }

	const update = asRecord(root.update) ?? root
	const sessionUpdate = asString(update.sessionUpdate)
	if (!sessionUpdate) return { messages, streamingText, didStream: false }

	const nextMessages = [...messages]
	let nextStreaming = streamingText
	let didStream = false

	if (sessionUpdate === "agent_message_chunk") {
		const chunk = extractTextFromContent(update.content)
		if (chunk) {
			// Paint into the assistant message immediately (not only a side buffer).
			const assistant = ensureAssistantMessage(nextMessages)
			assistant.content = mergeStreamText(assistant.content, chunk)
			nextStreaming = ""
			didStream = true
		}
		return { messages: nextMessages, streamingText: nextStreaming, didStream }
	}

	if (sessionUpdate === "agent_thought_chunk") {
		// Optional: ignore thought for main transcript (keeps UI clean).
		return { messages, streamingText, didStream: false }
	}

	if (sessionUpdate === "user_message_chunk") {
		// Live user text already added optimistically by the composer.
		return { messages, streamingText, didStream: false }
	}

	if (sessionUpdate === "tool_call" || sessionUpdate === "tool_call_update") {
		const assistant = ensureAssistantMessage(nextMessages)
		const tool = mapToolFromUpdate(update)
		const tools = [...assistant.toolCalls]
		const idx = tools.findIndex((t) => t.id === tool.id)
		if (idx >= 0) {
			tools[idx] = {
				...tools[idx],
				...tool,
				content: tool.content || tools[idx].content,
			}
		} else {
			tools.push(tool)
		}
		const last = nextMessages[nextMessages.length - 1]!
		nextMessages[nextMessages.length - 1] = { ...last, toolCalls: tools }
		didStream = true
		return { messages: nextMessages, streamingText: nextStreaming, didStream }
	}

	return { messages, streamingText, didStream: false }
}

export function appendStreamToMessages(
	messages: ChatMessage[],
	streamingText: string,
): ChatMessage[] {
	const stream = streamingText
	if (!stream.trim()) return messages
	const next = [...messages]
	const last = next[next.length - 1]
	if (last?.role === "assistant") {
		next[next.length - 1] = {
			...last,
			content: mergeStreamText(last.content, stream),
		}
		return next
	}
	next.push({
		id: crypto.randomUUID(),
		role: "assistant",
		content: stream,
		toolCalls: [],
		timestamp: Date.now(),
	})
	return next
}
