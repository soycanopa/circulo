import { useEffect, useState } from "react"
import { cn } from "@/lib/utils"

const THINKING_PHRASES = [
	"Consultando al oráculo…",
	"Cocinando la respuesta a fuego lento…",
	"Pidiendo permiso a los bits…",
	"Rebuscando en la RAM emocional…",
	"Negociando con el compilador…",
	"Haciendo magia negra (legal)…",
	"Preguntándole a Stack Overflow…",
	"Calentando las neuronas artificiales…",
	"Traduciendo del binario al humano…",
	"Sacando cuentas en el sótano del servidor…",
	"Persuadiendo a un if/else…",
	"Esperando que el WiFi coopere…",
	"Contando tokens como quien cuenta ovejas…",
	"Revisando si esto era un bug o feature…",
	"Invocando al espíritu de Turing…",
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
