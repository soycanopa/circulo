import type { ChatMessage, ToolCall } from "@/types/acp"

function textFromContent(content: unknown): string {
	if (!content) return ""
	if (typeof content === "string") return content
	if (typeof content === "object" && content !== null) {
		const obj = content as Record<string, unknown>
		if (typeof obj.text === "string") return obj.text
		if (Array.isArray(obj)) {
			return obj.map(textFromContent).join("")
		}
	}
	return ""
}

function extractUpdate(payload: unknown): Record<string, unknown> | null {
	if (!payload || typeof payload !== "object") return null
	const root = payload as Record<string, unknown>
	const update = root.update
	if (update && typeof update === "object") {
		return update as Record<string, unknown>
	}
	return root
}

function mapToolCall(raw: Record<string, unknown>): ToolCall {
	const id =
		(typeof raw.toolCallId === "string" && raw.toolCallId) ||
		(typeof raw.id === "string" && raw.id) ||
		crypto.randomUUID()
	const title =
		(typeof raw.title === "string" && raw.title) ||
		(typeof raw.kind === "string" && raw.kind) ||
		"tool"
	const status =
		(typeof raw.status === "string" && raw.status) ||
		(typeof raw.toolCallStatus === "string" && raw.toolCallStatus) ||
		"pending"
	const kind = typeof raw.kind === "string" ? raw.kind : undefined
	let content = ""
	if (typeof raw.content === "string") content = raw.content
	else if (raw.content) content = JSON.stringify(raw.content, null, 2)

	return {
		id,
		title,
		kind,
		status,
		content,
		rawInput: raw.rawInput ?? raw.input,
		rawOutput: raw.rawOutput ?? raw.output,
	}
}

export function applySessionUpdate(
	messages: ChatMessage[],
	streamingText: string,
	payload: unknown,
): { messages: ChatMessage[]; streamingText: string } {
	const update = extractUpdate(payload)
	if (!update) return { messages, streamingText }

	const sessionUpdate =
		typeof update.sessionUpdate === "string"
			? update.sessionUpdate
			: typeof update.session_update === "string"
				? update.session_update
				: null

	if (
		sessionUpdate === "agent_message_chunk" ||
		sessionUpdate === "agent_thought_chunk"
	) {
		const chunk = textFromContent(update.content ?? update.text)
		if (!chunk) return { messages, streamingText }
		// Skip pure thought for MVP streaming into main buffer (optional)
		if (sessionUpdate === "agent_thought_chunk") {
			return { messages, streamingText }
		}
		return { messages, streamingText: streamingText + chunk }
	}

	if (sessionUpdate === "user_message_chunk") {
		// History replay — ignore for live turns; local user messages already exist.
		return { messages, streamingText }
	}

	if (sessionUpdate === "tool_call" || sessionUpdate === "tool_call_update") {
		const raw =
			(update.toolCall as Record<string, unknown> | undefined) ??
			(update as Record<string, unknown>)
		const tool = mapToolCall(raw)
		const next = [...messages]
		let last = next[next.length - 1]
		if (!last || last.role !== "assistant") {
			last = {
				id: crypto.randomUUID(),
				role: "assistant",
				content: streamingText,
				toolCalls: [],
				timestamp: Date.now(),
			}
			next.push(last)
			streamingText = ""
		} else if (streamingText) {
			last = {
				...last,
				content: last.content + streamingText,
			}
			next[next.length - 1] = last
			streamingText = ""
		}

		const tools = [...last.toolCalls]
		const idx = tools.findIndex((t) => t.id === tool.id)
		if (idx >= 0) {
			tools[idx] = { ...tools[idx], ...tool }
		} else {
			tools.push(tool)
		}
		next[next.length - 1] = { ...last, toolCalls: tools }
		return { messages: next, streamingText }
	}

	return { messages, streamingText }
}

export function appendStreamToMessages(
	messages: ChatMessage[],
	streamingText: string,
): ChatMessage[] {
	const stream = streamingText.trim()
	if (!stream) return messages
	const next = [...messages]
	const last = next[next.length - 1]
	if (last?.role === "assistant") {
		next[next.length - 1] = {
			...last,
			content: last.content ? `${last.content}${streamingText}` : streamingText,
		}
		return next
	}
	next.push({
		id: crypto.randomUUID(),
		role: "assistant",
		content: streamingText,
		toolCalls: [],
		timestamp: Date.now(),
	})
	return next
}
