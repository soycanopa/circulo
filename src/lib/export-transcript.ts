import type { ChatMessage } from "@/types/acp"
import { toolContentToText } from "@/lib/acp-parser"
import { exportTranscript } from "@/lib/tauri"

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
				lines.push("```", toolContentToText(tool.content), "```", "")
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
	document.body.appendChild(anchor)
	anchor.click()
	anchor.remove()
	URL.revokeObjectURL(url)
}

/** Save via native dialog in Tauri; returns true when a file was written. */
export async function exportTranscriptMarkdown(
	title: string,
	messages: ChatMessage[],
): Promise<boolean> {
	const markdown = formatTranscriptMarkdown(title, messages)
	const filename = `${sanitizeFilename(title)}.md`
	return exportTranscript(filename, markdown)
}
