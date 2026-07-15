/** Normalize streamed plan text so markdown block elements render reliably. */
export function normalizePlanMarkdown(content: string): string {
	return content
		.replace(/\r\n/g, "\n")
		.replace(/```[\s\S]*?```/g, (block) => block.replace(/\n{3,}/g, "\n\n"))
		.replace(/([^\n])(#{1,6}\s)/g, "$1\n\n$2")
		.replace(/([^\n])(\d+\.\s)/g, "$1\n$2")
		.replace(/([^\n])([-*+]\s)/g, "$1\n$2")
		.replace(/\n{3,}/g, "\n\n")
		.trim()
}