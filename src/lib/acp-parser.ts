import type { ChatMessage, SessionInfo, ToolCallState } from "@/types/acp"

function asRecord(value: unknown): Record<string, unknown> | null {
	return value && typeof value === "object" ? (value as Record<string, unknown>) : null
}

function asString(value: unknown): string | undefined {
	return typeof value === "string" ? value : undefined
}

function extractTextFromContent(content: unknown): string {
	if (typeof content === "string") return content
	const record = asRecord(content)
	if (!record) return ""
	if (record.type === "text" && typeof record.text === "string") return record.text
	if (Array.isArray(content)) return content.map((item) => extractTextFromContent(item)).join("")
	return ""
}

function extractToolContent(content: unknown): { text: string; diff?: ToolCallState["diff"] } {
	if (!Array.isArray(content)) return { text: extractTextFromContent(content) }
	let text = ""
	let diff: ToolCallState["diff"] | undefined
	for (const item of content) {
		const record = asRecord(item)
		if (!record) continue
		if (record.type === "content") text += extractTextFromContent(record.content)
		if (record.type === "diff") {
			diff = {
				path: asString(record.path) ?? "unknown",
				oldText: asString(record.oldText),
				newText: asString(record.newText) ?? "",
			}
		}
	}
	return { text, diff }
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

function ensureUserMessage(messages: ChatMessage[]): ChatMessage {
	const last = messages[messages.length - 1]
	if (last?.role === "user") return last

	const created: ChatMessage = {
		id: crypto.randomUUID(),
		role: "user",
		content: "",
		toolCalls: [],
		timestamp: Date.now(),
	}
	messages.push(created)
	return created
}

export interface ApplySessionUpdateOptions {
	/** When false, agent chunks only update streamingText (avoids duplicate bubble). */
	streamToMessage?: boolean
}

export function applySessionUpdate(
	messages: ChatMessage[],
	streamingText: string,
	payload: unknown,
	options: ApplySessionUpdateOptions = {},
): { messages: ChatMessage[]; streamingText: string } {
	const { streamToMessage = true } = options
	const root = asRecord(payload)
	const update = asRecord(root?.update)
	if (!update) return { messages, streamingText }

	const sessionUpdate = asString(update.sessionUpdate)
	const nextMessages = [...messages]
	let nextStreaming = streamingText

	if (sessionUpdate === "user_message_chunk") {
		const user = ensureUserMessage(nextMessages)
		const chunk = extractTextFromContent(update.content)
		user.content = mergeStreamText(user.content, chunk)
	}

	if (sessionUpdate === "agent_message_chunk") {
		const chunk = extractTextFromContent(update.content)
		if (streamToMessage) {
			const assistant = ensureAssistantMessage(nextMessages)
			assistant.content = mergeStreamText(assistant.content, chunk)
		} else {
			nextStreaming = mergeStreamText(nextStreaming, chunk)
		}
	}

	if (sessionUpdate === "tool_call") {
		const assistant = ensureAssistantMessage(nextMessages)
		const toolCallId = asString(update.toolCallId) ?? crypto.randomUUID()
		if (assistant.toolCalls.some((tool) => tool.id === toolCallId)) {
			return { messages: nextMessages, streamingText: nextStreaming }
		}
		const { text, diff } = extractToolContent(update.content)
		assistant.toolCalls.push({
			id: toolCallId,
			title: asString(update.title) ?? "Tool call",
			kind: asString(update.kind),
			status: (asString(update.status) as ToolCallState["status"]) ?? "pending",
			content: text,
			diff,
			rawInput: update.rawInput,
			rawOutput: update.rawOutput,
		})
	}

	if (sessionUpdate === "tool_call_update") {
		const assistant = ensureAssistantMessage(nextMessages)
		const toolCallId = asString(update.toolCallId)
		if (!toolCallId) return { messages: nextMessages, streamingText: nextStreaming }

		const existing = assistant.toolCalls.find((tool) => tool.id === toolCallId)
		if (existing) {
			if (update.title) existing.title = asString(update.title) ?? existing.title
			if (update.status) existing.status = asString(update.status) as ToolCallState["status"]
			const { text, diff } = extractToolContent(update.content)
			if (text) existing.content = `${existing.content}${text}`
			if (diff) existing.diff = diff
			if (update.rawOutput) existing.rawOutput = update.rawOutput
		} else {
			const { text, diff } = extractToolContent(update.content)
			assistant.toolCalls.push({
				id: toolCallId,
				title: asString(update.title) ?? "Tool call",
				kind: asString(update.kind),
				status: (asString(update.status) as ToolCallState["status"]) ?? "in_progress",
				content: text,
				diff,
				rawOutput: update.rawOutput,
			})
		}
	}

	return { messages: nextMessages, streamingText: nextStreaming }
}

export function applySessionInfoUpdate(
	sessions: SessionInfo[],
	payload: unknown,
): SessionInfo[] {
	const root = asRecord(payload)
	const update = asRecord(root?.update)
	const sessionId = asString(root?.sessionId)
	if (!update || !sessionId) return sessions

	return sessions.map((session) => {
		if (session.sessionId !== sessionId) return session
		const title = update.title
		const updatedAt = update.updatedAt
		return {
			...session,
			title: typeof title === "string" ? title : session.title,
			updatedAt: typeof updatedAt === "string" ? updatedAt : session.updatedAt,
		}
	})
}