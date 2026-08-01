import type { ChatMessage } from "@/types/acp"

function sanitizeFilename(name: string): string {
	const base = name.trim().slice(0, 64) || "chat"
	return base.replace(/[^\w.-]+/g, "-").replace(/-+/g, "-")
}

export function formatTranscriptMarkdown(
	title: string,
	messages: ChatMessage[],
): string {
	const lines = [`# ${title}`, "", `Exported ${new Date().toISOString()}`, ""]

	for (const message of messages) {
		if (!message.content.trim() && message.toolCalls.length === 0) continue
		const heading = message.role === "user" ? "## User" : "## Assistant"
		lines.push(heading, "")
		if (message.content.trim()) {
			lines.push(message.content.trim(), "")
		}
		for (const tool of message.toolCalls) {
			lines.push(`### Tool: ${tool.title} (${tool.status})`, "")
			if (tool.content) {
				lines.push("```", tool.content, "```", "")
			}
		}
	}

	return `${lines.join("\n").trim()}\n`
}

export function downloadTextFile(filename: string, content: string): void {
	const blob = new Blob([content], { type: "text/plain;charset=utf-8" })
	const url = URL.createObjectURL(blob)
	const anchor = document.createElement("a")
	anchor.href = url
	anchor.download = filename
	anchor.click()
	URL.revokeObjectURL(url)
}

export function exportTranscriptMarkdown(
	title: string,
	messages: ChatMessage[],
): void {
	const markdown = formatTranscriptMarkdown(title, messages)
	downloadTextFile(`${sanitizeFilename(title)}.md`, markdown)
}
