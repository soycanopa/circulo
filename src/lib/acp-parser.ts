import type { ChatMessage, ToolCall, ToolCallContent, ToolCallDiff } from "@/types/acp"

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

function extractDiffContent(item: Record<string, unknown>): ToolCallDiff | null {
	const path = asString(item.path) ?? asString(item.filePath)
	if (!path) return null
	const oldText =
		asString(item.oldText) ?? asString(item.old_text) ?? asString(item.before) ?? ""
	const newText =
		asString(item.newText) ?? asString(item.new_text) ?? asString(item.after) ?? ""
	return { type: "diff", path, oldText, newText }
}

function diffToText(diff: ToolCallDiff): string {
	return `[diff ${diff.path}]\n--- old\n${diff.oldText}\n+++ new\n${diff.newText}`
}

export function toolContentToText(content: ToolCallContent | undefined): string {
	if (!content) return ""
	return typeof content === "string" ? content : diffToText(content)
}

function extractToolContent(content: unknown): ToolCallContent {
	if (!Array.isArray(content)) return extractTextFromContent(content)
	let combined = ""
	const diffs: ToolCallDiff[] = []
	for (const item of content) {
		const record = asRecord(item)
		if (!record) {
			combined += extractTextFromContent(item)
			continue
		}
		if (record.type === "content") combined += extractTextFromContent(record.content)
		else if (record.type === "diff") {
			const diff = extractDiffContent(record)
			if (diff) diffs.push(diff)
		} else combined += extractTextFromContent(item)
	}
	if (diffs.length === 0) return combined
	if (diffs.length === 1) return diffs[0]!
	return diffs.map(diffToText).join("\n\n")
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

function findAssistantWithTool(
	messages: ChatMessage[],
	toolId: string,
): number {
	for (let i = messages.length - 1; i >= 0; i--) {
		const message = messages[i]
		if (
			message?.role === "assistant" &&
			message.toolCalls.some((tool) => tool.id === toolId)
		) {
			return i
		}
	}
	return -1
}

/** Assistant bubble with text only that follows a tool-bearing turn. */
function isPostToolContentBubble(
	messages: ChatMessage[],
	index: number,
): boolean {
	const message = messages[index]
	if (message?.role !== "assistant" || message.toolCalls.length > 0) {
		return false
	}
	for (let i = index - 1; i >= 0; i--) {
		const previous = messages[i]
		if (previous?.role !== "assistant") continue
		return previous.toolCalls.length > 0
	}
	return false
}

function appendChunkToAssistantMessages(
	messages: ChatMessage[],
	chunk: string,
): ChatMessage[] {
	const next = [...messages]
	const lastIdx = next.length - 1
	const last = next[lastIdx]

	if (last?.role === "assistant" && last.toolCalls.length > 0) {
		const follow = next[lastIdx + 1]
		if (follow?.role === "assistant" && follow.toolCalls.length === 0) {
			next[lastIdx + 1] = {
				...follow,
				content: mergeStreamText(follow.content, chunk),
			}
			return next
		}
		next.push({
			id: crypto.randomUUID(),
			role: "assistant",
			content: chunk,
			toolCalls: [],
			timestamp: Date.now(),
		})
		return next
	}

	if (last?.role === "assistant") {
		next[lastIdx] = {
			...last,
			content: mergeStreamText(last.content, chunk),
		}
		return next
	}

	next.push({
		id: crypto.randomUUID(),
		role: "assistant",
		content: chunk,
		toolCalls: [],
		timestamp: Date.now(),
	})
	return next
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
			const nextWithChunk = appendChunkToAssistantMessages(nextMessages, chunk)
			return {
				messages: nextWithChunk,
				streamingText: "",
				didStream: true,
			}
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
		const tool = mapToolFromUpdate(update)
		const existingIdx = findAssistantWithTool(nextMessages, tool.id)
		const lastIdx = nextMessages.length - 1
		const last = nextMessages[lastIdx]

		let targetIdx = existingIdx
		if (targetIdx < 0) {
			if (
				last?.role === "assistant" &&
				!isPostToolContentBubble(nextMessages, lastIdx)
			) {
				targetIdx = lastIdx
			} else {
				nextMessages.push({
					id: crypto.randomUUID(),
					role: "assistant",
					content: "",
					toolCalls: [],
					timestamp: Date.now(),
				})
				targetIdx = nextMessages.length - 1
			}
		}

		const base = nextMessages[targetIdx]!
		const tools = [...base.toolCalls]
		const tIdx = tools.findIndex((t) => t.id === tool.id)
		if (tIdx >= 0) {
			tools[tIdx] = {
				...tools[tIdx],
				...tool,
				content: tool.content || tools[tIdx].content,
			}
		} else {
			tools.push(tool)
		}
		nextMessages[targetIdx] = { ...base, toolCalls: tools }
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
	return appendChunkToAssistantMessages(messages, stream)
}
