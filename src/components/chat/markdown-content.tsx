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
					ul({ className: listClassName, children, ...props }) {
						const isTaskList = listClassName?.includes("contains-task-list")
						return (
							<ul
								{...props}
								className={cn(
									listClassName,
									isTaskList &&
										"my-3 list-none space-y-1.5 rounded-md border border-border/60 bg-card/30 p-3 pl-2",
								)}
							>
								{children}
							</ul>
						)
					},
					li({ className: itemClassName, children, ...props }) {
						const isTaskItem = itemClassName?.includes("task-list-item")
						return (
							<li
								{...props}
								className={cn(
									itemClassName,
									isTaskItem && "flex items-start gap-2.5 leading-snug",
								)}
							>
								{children}
							</li>
						)
					},
					input({ type, checked, disabled, ...props }) {
						if (type === "checkbox") {
							return (
								<input
									{...props}
									type="checkbox"
									checked={checked}
									disabled={disabled ?? true}
									readOnly
									className="mt-0.5 size-3.5 shrink-0 rounded border-border accent-diff-addition"
								/>
							)
						}
						return <input type={type} checked={checked} disabled={disabled} {...props} />
					},
					table({ children, ...props }) {
						return (
							<div className="my-3 overflow-x-auto rounded-md border border-border/60">
								<table
									{...props}
									className="w-full min-w-full border-collapse text-left text-xs"
								>
									{children}
								</table>
							</div>
						)
					},
					th({ children, ...props }) {
						return (
							<th
								{...props}
								className="border-b border-border/60 bg-muted/40 px-3 py-2 font-medium text-foreground"
							>
								{children}
							</th>
						)
					},
					td({ children, ...props }) {
						return (
							<td {...props} className="border-b border-border/40 px-3 py-2 text-muted-foreground">
								{children}
							</td>
						)
					},
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