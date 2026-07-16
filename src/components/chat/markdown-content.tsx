import ReactMarkdown from "react-markdown"
import remarkGfm from "remark-gfm"
import { CodeBlock } from "@/components/chat/code-block"
import { MarkdownDiffBlock } from "@/components/chat/markdown-diff-block"
import { cn } from "@/lib/utils"

interface MarkdownContentProps {
	content: string
	className?: string
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
							const language = match[1].toLowerCase()
							if (language === "diff") {
								return <MarkdownDiffBlock code={code} />
							}
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