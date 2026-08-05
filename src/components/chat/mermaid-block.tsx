import { useEffect, useId, useRef, useState } from "react"
import { cn } from "@/lib/utils"

interface MermaidBlockProps {
	code: string
	className?: string
}

/** Lazy-loaded mermaid renderer. Kept out of the main bundle on purpose. */
async function renderMermaid(id: string, code: string): Promise<string> {
	const mod = await import("mermaid")
	const mermaid = mod.default
	mermaid.initialize({
		startOnLoad: false,
		theme: "dark",
		fontFamily: "ui-sans-serif, system-ui, sans-serif",
		securityLevel: "strict",
	})
	const { svg } = await mermaid.render(id, code)
	return svg
}

/**
 * Renders a ```mermaid code block as a diagram. If the snippet is invalid or
 * the agent is still streaming an incomplete diagram, we fall back to the raw
 * code so the text is never lost.
 */
export function MermaidBlock({ code, className }: MermaidBlockProps) {
	const id = useId().replace(/:/g, "")
	const [svg, setSvg] = useState<string | null>(null)
	const [error, setError] = useState<string | null>(null)
	const requestRef = useRef(0)

	useEffect(() => {
		const request = ++requestRef.current
		let cancelled = false
		renderMermaid(id, code)
			.then((next) => {
				if (cancelled || request !== requestRef.current) return
				setSvg(next)
				setError(null)
			})
			.catch((err: unknown) => {
				if (cancelled || request !== requestRef.current) return
				setSvg(null)
				setError(err instanceof Error ? err.message : String(err))
			})
		return () => {
			cancelled = true
		}
	}, [code, id])

	return (
		<div
			className={cn(
				"my-2 overflow-x-auto rounded-md border border-border bg-black/20 p-3",
				className,
			)}
		>
			{svg ? (
				<div dangerouslySetInnerHTML={{ __html: svg }} />
			) : (
				<pre className="font-mono text-[12px] leading-relaxed text-fg/95">
					{error ? (
						<span className="block text-[10px] uppercase tracking-wider text-amber-300/80">
							Mermaid render failed
						</span>
					) : null}
					<code>{code}</code>
				</pre>
			)}
		</div>
	)
}
