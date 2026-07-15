import { useEffect, useState } from "react"
import ReactMarkdown from "react-markdown"
import remarkGfm from "remark-gfm"
import { codeToHtml } from "shiki"
import { cn } from "@/lib/utils"

interface MarkdownContentProps {
	content: string
	className?: string
}

function CodeBlock({ language, code }: { language: string; code: string }) {
	const [html, setHtml] = useState<string>("")

	useEffect(() => {
		let cancelled = false
		codeToHtml(code, {
			lang: language || "text",
			theme: "github-dark-default",
		})
			.then((result) => {
				if (!cancelled) setHtml(result)
			})
			.catch(() => {
				if (!cancelled) {
					setHtml(`<pre><code>${code}</code></pre>`)
				}
			})

		return () => {
			cancelled = true
		}
	}, [code, language])

	return (
		<div
			className="my-2 overflow-x-auto rounded-md border border-border text-xs"
			dangerouslySetInnerHTML={{ __html: html }}
		/>
	)
}

export function MarkdownContent({ content, className }: MarkdownContentProps) {
	return (
		<div className={cn("prose prose-invert max-w-none text-sm leading-relaxed", className)}>
			<ReactMarkdown
				remarkPlugins={[remarkGfm]}
				components={{
					code({ className: codeClassName, children }) {
						const match = /language-(\w+)/.exec(codeClassName ?? "")
						const code = String(children).replace(/\n$/, "")
						if (match) {
							return <CodeBlock language={match[1]} code={code} />
						}
						return (
							<code className="rounded bg-muted px-1 py-0.5 font-mono text-xs">
								{children}
							</code>
						)
					},
					pre({ children }) {
						return <>{children}</>
					},
				}}
			>
				{content}
			</ReactMarkdown>
		</div>
	)
}