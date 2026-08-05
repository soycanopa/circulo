import { useEffect, useState } from "react"
import { cn } from "@/lib/utils"

const THINKING_PHRASES = [
	"Consulting the oracle…",
	"Simmering the answer…",
	"Asking the bits for permission…",
	"Digging through emotional RAM…",
	"Negotiating with the compiler…",
	"Doing (legal) black magic…",
	"Checking Stack Overflow…",
	"Warming up the artificial neurons…",
	"Translating from binary to human…",
	"Doing the math in the server basement…",
	"Persuading an if/else…",
	"Hoping the WiFi cooperates…",
	"Counting tokens like sheep…",
	"Checking if it was a bug or a feature…",
	"Invoking the spirit of Turing…",
] as const

interface ThinkingShimmerProps {
	className?: string
}

export function ThinkingShimmer({ className }: ThinkingShimmerProps) {
	const [index, setIndex] = useState(
		() => Math.floor(Math.random() * THINKING_PHRASES.length),
	)

	useEffect(() => {
		const id = window.setInterval(() => {
			setIndex((current) => (current + 1) % THINKING_PHRASES.length)
		}, 2800)
		return () => window.clearInterval(id)
	}, [])

	return (
		<p
			className={cn("shimmer text-sm text-muted", className)}
			role="status"
			aria-live="polite"
		>
			{THINKING_PHRASES[index]}
		</p>
	)
}
