import type { ReactNode } from "react"

type Block =
	| { type: "paragraph"; text: string }
	| { type: "code"; lang: string; text: string }

function parseBlocks(text: string): Block[] {
	const blocks: Block[] = []
	const fence = /```(\w*)\n?([\s\S]*?)```/g
	let last = 0
	let match: RegExpExecArray | null

	while ((match = fence.exec(text)) !== null) {
		const before = text.slice(last, match.index)
		if (before.trim()) blocks.push({ type: "paragraph", text: before })
		blocks.push({
			type: "code",
			lang: match[1] ?? "",
			text: match[2]?.replace(/\n$/, "") ?? "",
		})
		last = match.index + match[0].length
	}

	const tail = text.slice(last)
	if (tail.trim() || blocks.length === 0) {
		blocks.push({ type: "paragraph", text: tail })
	}
	return blocks
}

function renderInline(text: string): ReactNode[] {
	const parts: ReactNode[] = []
	const pattern = /(`[^`]+`|\*\*[^*]+\*\*)/g
	let last = 0
	let match: RegExpExecArray | null
	let key = 0

	while ((match = pattern.exec(text)) !== null) {
		if (match.index > last) {
			parts.push(text.slice(last, match.index))
		}
		const token = match[0]
		if (token.startsWith("`")) {
			parts.push(
				<code
					key={key++}
					className="rounded bg-white/10 px-1 py-0.5 font-mono text-[0.9em]"
				>
					{token.slice(1, -1)}
				</code>,
			)
		} else {
			parts.push(
				<strong key={key++} className="font-semibold text-fg">
					{token.slice(2, -2)}
				</strong>,
			)
		}
		last = match.index + token.length
	}

	if (last < text.length) parts.push(text.slice(last))
	return parts
}

interface SimpleMarkdownProps {
	text: string
	className?: string
}

/** Minimal markdown for assistant messages: fences, `code`, **bold**. */
export function SimpleMarkdown({ text, className }: SimpleMarkdownProps) {
	const blocks = parseBlocks(text)

	return (
		<div className={className}>
			{blocks.map((block, index) => {
				if (block.type === "code") {
					return (
						<pre
							key={index}
							className="my-2 overflow-x-auto rounded-md border border-border bg-black/30 px-3 py-2 font-mono text-[12px] leading-relaxed text-fg/95"
						>
							{block.lang ? (
								<div className="mb-1 text-[10px] uppercase tracking-wider text-muted">
									{block.lang}
								</div>
							) : null}
							<code>{block.text}</code>
						</pre>
					)
				}
				const lines = block.text.split("\n")
				return (
					<p key={index} className="whitespace-pre-wrap">
						{lines.map((line, lineIndex) => (
							<span key={lineIndex}>
								{lineIndex > 0 ? <br /> : null}
								{renderInline(line)}
							</span>
						))}
					</p>
				)
			})}
		</div>
	)
}
