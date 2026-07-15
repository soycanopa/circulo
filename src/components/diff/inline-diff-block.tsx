import { cn } from "@/lib/utils"

interface InlineDiffBlockProps {
	path: string
	oldText?: string
	newText: string
}

function buildUnifiedLines(oldText: string | undefined, newText: string) {
	const oldLines = (oldText ?? "").split("\n")
	const newLines = newText.split("\n")
	const lines: Array<{ type: "add" | "remove" | "same"; text: string }> = []

	const max = Math.max(oldLines.length, newLines.length)
	for (let index = 0; index < max; index += 1) {
		const oldLine = oldLines[index]
		const newLine = newLines[index]

		if (oldLine === newLine) {
			if (newLine !== undefined) lines.push({ type: "same", text: newLine })
			continue
		}

		if (oldLine !== undefined) lines.push({ type: "remove", text: oldLine })
		if (newLine !== undefined) lines.push({ type: "add", text: newLine })
	}

	return lines
}

export function InlineDiffBlock({ path, oldText, newText }: InlineDiffBlockProps) {
	const lines = buildUnifiedLines(oldText, newText)

	return (
		<div className="my-2 overflow-hidden rounded-md border border-border">
			<div className="border-b border-border bg-muted/50 px-3 py-1.5 font-mono text-xs text-muted-foreground">
				{path}
			</div>
			<pre className="scrollbar-thin max-h-64 overflow-auto p-2 font-mono text-xs leading-5">
				{lines.map((line, index) => (
					<div
						key={`${line.type}-${index}`}
						className={cn(
							line.type === "add" && "bg-diff-addition/10 text-diff-addition",
							line.type === "remove" && "bg-diff-deletion/10 text-diff-deletion",
							line.type === "same" && "text-foreground/80",
						)}
					>
						<span className="mr-2 inline-block w-4 opacity-60">
							{line.type === "add" ? "+" : line.type === "remove" ? "-" : " "}
						</span>
						{line.text || " "}
					</div>
				))}
			</pre>
		</div>
	)
}