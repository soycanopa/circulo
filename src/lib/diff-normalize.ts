/** Normalize LLM or partial diff text into a unified patch Pierre can parse. */
export function ensureUnifiedDiffFormat(content: string, filePath = "file"): string {
	const text = content.replace(/\r\n/g, "\n").trim()
	if (!text) return text

	if (/^diff --git /m.test(text) || /^--- /m.test(text) || /^\+\+\+ /m.test(text)) {
		return text
	}

	const safePath = filePath.replace(/\s+/g, "_") || "file"

	if (/^@@ /m.test(text)) {
		return `--- a/${safePath}\n+++ b/${safePath}\n${text}`
	}

	const lines = text.split("\n")
	const hasChangeLines = lines.some(
		(line) => line.startsWith("+") || line.startsWith("-") || line.startsWith(" "),
	)

	if (!hasChangeLines) return text

	const body = lines
		.map((line) => {
			if (line.startsWith("+") || line.startsWith("-") || line.startsWith(" ")) return line
			if (line.startsWith("@@")) return line
			return ` ${line}`
		})
		.join("\n")

	if (body.includes("@@")) {
		return `--- a/${safePath}\n+++ b/${safePath}\n${body}`
	}

	const deletions = lines.filter((line) => line.startsWith("-")).length
	const additions = lines.filter((line) => line.startsWith("+")).length
	const oldCount = Math.max(deletions, 1)
	const newCount = Math.max(additions, 1)

	return `--- a/${safePath}\n+++ b/${safePath}\n@@ -1,${oldCount} +1,${newCount} @@\n${body}`
}