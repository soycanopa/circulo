import type { ReactNode } from "react"

export type MarkdownBlock =
	| { type: "paragraph"; text: string }
	| { type: "code"; lang: string; text: string }
	| { type: "heading"; level: number; text: string }
	| { type: "list"; ordered: boolean; items: string[] }

export function parseMarkdownBlocks(text: string): MarkdownBlock[] {
	const blocks: MarkdownBlock[] = []
	const fence = /```(\w*)\n?([\s\S]*?)```/g
	let last = 0
	let match: RegExpExecArray | null

	while ((match = fence.exec(text)) !== null) {
		const before = text.slice(last, match.index)
		if (before.trim()) pushTextBlocks(blocks, before)
		blocks.push({
			type: "code",
			lang: match[1] ?? "",
			text: match[2]?.replace(/\n$/, "") ?? "",
		})
		last = match.index + match[0].length
	}

	const tail = text.slice(last)
	if (tail.trim() || blocks.length === 0) pushTextBlocks(blocks, tail)
	return blocks
}

function pushTextBlocks(blocks: MarkdownBlock[], text: string) {
	const chunks = text.split(/\n{2,}/)
	for (const chunk of chunks) {
		const trimmed = chunk.trim()
		if (!trimmed) continue

		const lines = trimmed.split("\n")
		const bulletLines = lines.filter((line) => /^[-*]\s+/.test(line))
		if (bulletLines.length === lines.length && bulletLines.length > 0) {
			blocks.push({
				type: "list",
				ordered: false,
				items: bulletLines.map((line) => line.replace(/^[-*]\s+/, "")),
			})
			continue
		}

		const orderedLines = lines.filter((line) => /^\d+\.\s+/.test(line))
		if (orderedLines.length === lines.length && orderedLines.length > 0) {
			blocks.push({
				type: "list",
				ordered: true,
				items: orderedLines.map((line) => line.replace(/^\d+\.\s+/, "")),
			})
			continue
		}

		if (lines.length === 1) {
			const heading = lines[0].match(/^(#{1,3})\s+(.+)$/)
			if (heading) {
				blocks.push({
					type: "heading",
					level: heading[1].length,
					text: heading[2],
				})
				continue
			}
		}

		blocks.push({ type: "paragraph", text: trimmed })
	}
}

function renderInline(text: string): ReactNode[] {
	const parts: ReactNode[] = []
	const pattern = /(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\))/g
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
		} else if (token.startsWith("**")) {
			parts.push(
				<strong key={key++} className="font-semibold text-fg">
					{token.slice(2, -2)}
				</strong>,
			)
		} else {
			const link = token.match(/^\[([^\]]+)\]\(([^)]+)\)$/)
			if (link) {
				parts.push(
					<a
						key={key++}
						href={link[2]}
						target="_blank"
						rel="noreferrer"
						className="text-sky-300 underline-offset-2 hover:underline"
					>
						{link[1]}
					</a>,
				)
			} else {
				parts.push(token)
			}
		}
		last = match.index + token.length
	}

	if (last < text.length) parts.push(text.slice(last))
	return parts
}

function renderParagraph(text: string, key: string | number) {
	const lines = text.split("\n")
	return (
		<p key={key} className="whitespace-pre-wrap">
			{lines.map((line, lineIndex) => (
				<span key={lineIndex}>
					{lineIndex > 0 ? <br /> : null}
					{renderInline(line)}
				</span>
			))}
		</p>
	)
}

interface SimpleMarkdownProps {
	text: string
	className?: string
}

/** Lightweight markdown for assistant messages. */
export function SimpleMarkdown({ text, className }: SimpleMarkdownProps) {
	const blocks = parseMarkdownBlocks(text)

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
				if (block.type === "heading") {
					const Tag = block.level === 1 ? "h3" : block.level === 2 ? "h4" : "h5"
					return (
						<Tag
							key={index}
							className="mb-1 mt-3 font-semibold text-fg first:mt-0"
						>
							{renderInline(block.text)}
						</Tag>
					)
				}
				if (block.type === "list") {
					const ListTag = block.ordered ? "ol" : "ul"
					return (
						<ListTag
							key={index}
							className={
								block.ordered
									? "my-2 list-decimal space-y-1 pl-5"
									: "my-2 list-disc space-y-1 pl-5"
							}
						>
							{block.items.map((item, itemIndex) => (
								<li key={itemIndex}>{renderInline(item)}</li>
							))}
						</ListTag>
					)
				}
				return renderParagraph(block.text, index)
			})}
		</div>
	)
}
