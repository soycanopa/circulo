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

/** True when streamed text looks like a structured plan, not a casual reply. */
export function isPlanLikeContent(content: string): boolean {
	const text = normalizePlanMarkdown(content)
	if (!text) return false

	if (
		/\b(plan(?:\s+(?:propuesto|de\s+implementación|overview))?|implementation\s+plan)\b/i.test(
			text,
		)
	) {
		return true
	}

	const hasHeading = /^#{1,3}\s+\S/m.test(text)
	const numberedItems = (text.match(/^\d+\.\s+\S/gm) ?? []).length
	const bulletItems = (text.match(/^[-*+]\s+\S/gm) ?? []).length

	if (hasHeading && (numberedItems >= 2 || bulletItems >= 3)) return true
	if (numberedItems >= 4) return true
	if (hasHeading && text.length >= 120) return true

	return false
}