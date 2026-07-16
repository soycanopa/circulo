import { Check, Copy } from "lucide-react"
import { useEffect, useState } from "react"
import { highlightCode, normalizeHighlightLanguage } from "@/lib/shiki-cache"
import { cn } from "@/lib/utils"

export type CodeBlockVariant = "full" | "minimal" | "terminal"

interface CodeBlockProps {
	language?: string
	code: string
	variant?: CodeBlockVariant
	className?: string
}

const VARIANT_STYLES: Record<CodeBlockVariant, string> = {
	full: "my-2 overflow-hidden rounded-md border border-border",
	minimal: "my-2 overflow-hidden rounded-md",
	terminal: "my-2 overflow-hidden rounded-md border border-border bg-[#141414]",
}

export function CodeBlock({
	language,
	code,
	variant = "full",
	className,
}: CodeBlockProps) {
	const [html, setHtml] = useState("")
	const [copied, setCopied] = useState(false)
	const displayLanguage = normalizeHighlightLanguage(language)

	useEffect(() => {
		let cancelled = false
		void highlightCode(code, displayLanguage).then((result) => {
			if (!cancelled) setHtml(result)
		})
		return () => {
			cancelled = true
		}
	}, [code, displayLanguage])

	async function handleCopy() {
		try {
			await navigator.clipboard.writeText(code)
			setCopied(true)
			window.setTimeout(() => setCopied(false), 1600)
		} catch {
			setCopied(false)
		}
	}

	return (
		<div className={cn("group/code relative", VARIANT_STYLES[variant], className)}>
			{variant !== "minimal" ? (
				<div className="flex items-center justify-between gap-2 border-b border-border/60 bg-muted/30 px-2.5 py-1">
					<span className="font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
						{displayLanguage}
					</span>
					<button
						type="button"
						onClick={() => void handleCopy()}
						className="inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
						aria-label="Copiar código"
					>
						{copied ? <Check className="size-3" /> : <Copy className="size-3" />}
						{copied ? "Copiado" : "Copiar"}
					</button>
				</div>
			) : (
				<button
					type="button"
					onClick={() => void handleCopy()}
					className="absolute right-1.5 top-1.5 z-10 inline-flex items-center gap-1 rounded bg-background/80 px-1.5 py-0.5 text-[10px] text-muted-foreground opacity-0 transition-opacity hover:text-foreground group-hover/code:opacity-100"
					aria-label="Copiar código"
				>
					{copied ? <Check className="size-3" /> : <Copy className="size-3" />}
				</button>
			)}
			<div
				className={cn(
					"overflow-x-auto text-xs [&_pre]:m-0 [&_pre]:bg-transparent [&_pre]:p-2.5",
					variant === "terminal" && "[&_pre]:p-3",
				)}
				dangerouslySetInnerHTML={{ __html: html }}
			/>
		</div>
	)
}