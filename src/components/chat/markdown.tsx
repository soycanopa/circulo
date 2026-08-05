import { useMemo, type ReactNode } from "react"
import ReactMarkdown, { type Components, type ExtraProps } from "react-markdown"
import rehypeHighlight from "rehype-highlight"
import rehypeKatex from "rehype-katex"
import remarkGfm from "remark-gfm"
import remarkMath from "remark-math"
import { MermaidBlock } from "@/components/chat/mermaid-block"
import { cn } from "@/lib/utils"

function codeText(node: ExtraProps["node"]): string {
	const children = node?.children ?? []
	return children
		.map((child) => ("value" in child ? child.value : ""))
		.join("")
}

function languageFromNode(node: ExtraProps["node"]): string {
	const className = node?.properties?.className
	if (typeof className !== "string") return ""
	const match = /language-(\w+)/.exec(className)
	return match?.[1] ?? ""
}

const components: Components = {
	a: ({ href, children, ...props }) => (
		<a
			href={href}
			target="_blank"
			rel="noreferrer"
			className="text-sky-300 underline-offset-2 hover:underline"
			{...props}
		>
			{children}
		</a>
	),
	table: ({ children, ...props }) => (
		<div className="my-2 w-full overflow-x-auto rounded-md border border-border">
			<table
				className="w-full border-collapse text-[13px] leading-relaxed text-fg"
				{...props}
			>
				{children}
			</table>
		</div>
	),
	thead: ({ children, ...props }) => (
		<thead
			className="bg-white/[0.06] text-left text-[11px] uppercase tracking-wide text-muted"
			{...props}
		>
			{children}
		</thead>
	),
	th: ({ children, ...props }) => (
		<th className="border-b border-border px-3 py-2 font-medium" {...props}>
			{children}
		</th>
	),
	td: ({ children, ...props }) => (
		<td className="border-b border-border/70 px-3 py-2 align-top" {...props}>
			{children}
		</td>
	),
	tr: ({ children, ...props }) => (
		<tr className="odd:bg-white/[0.02] even:bg-transparent" {...props}>
			{children}
		</tr>
	),
	input: ({ type, checked, ...props }) =>
		type === "checkbox" ? (
			<input
				type="checkbox"
				checked={checked}
				disabled
				className="mr-1.5 inline-block size-3.5 shrink-0 accent-emerald-400"
				{...props}
			/>
		) : (
			<input type={type} {...props} />
		),
	code: ({ className, children, ...props }) => {
		const isInline = !className
		if (isInline) {
			return (
				<code
					className="rounded bg-white/10 px-1 py-0.5 font-mono text-[0.9em]"
					{...props}
				>
					{children}
				</code>
			)
		}
		return (
			<code className={cn("hljs", className)} {...props}>
				{children}
			</code>
		)
	},
	pre: ({ node, children }: ExtraProps & { children?: ReactNode }) => {
		if (languageFromNode(node) === "mermaid") {
			return <MermaidBlock code={codeText(node)} />
		}
		const lang = languageFromNode(node)
		return (
			<div className="my-2 overflow-hidden rounded-md border border-border bg-black/30">
				{lang ? (
					<div className="border-b border-border/60 px-3 py-1 text-[10px] uppercase tracking-wider text-muted">
						{lang}
					</div>
				) : null}
				<pre className="overflow-x-auto p-3 font-mono text-[12px] leading-relaxed text-fg/95">
					{children}
				</pre>
			</div>
		)
	},
	hr: () => <hr className="my-4 border-border" />,
	blockquote: ({ children, ...props }) => (
		<blockquote
			className="my-2 border-l-2 border-accent/50 pl-3 text-muted"
			{...props}
		>
			{children}
		</blockquote>
	),
}

interface MarkdownProps {
	text: string
	className?: string
}

/** Rich Markdown renderer for assistant messages (GFM, KaTeX, highlight, mermaid). */
export function Markdown({ text, className }: MarkdownProps) {
	const markdown = useMemo(
		() => (
			<ReactMarkdown
				remarkPlugins={[remarkGfm, remarkMath]}
				rehypePlugins={[
					rehypeKatex,
					[rehypeHighlight, { plainText: ["mermaid"] }],
				]}
				components={components}
			>
				{text}
			</ReactMarkdown>
		),
		[text],
	)

	return <div className={cn("markdown-body", className)}>{markdown}</div>
}
