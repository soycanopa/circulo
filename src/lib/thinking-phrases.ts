import type { TurnPhase } from "@/lib/turn-phase"

export const THINKING_PHRASES = [
	"Convenciendo a los electrones…",
	"Pidiéndole permiso al compilador…",
	"Contando los bugs antes de que existan…",
	"Negociando con el stack trace…",
	"Revisando si el café ya compiló…",
	"Persuadiendo a TypeScript…",
	"Buscando el punto y coma perdido…",
	"Consultando al pato de goma…",
	"Ordenando los cables imaginarios…",
	"Calentando los transistores…",
	"Preguntándole a la IA qué piensa la IA…",
	"Traduciendo del devés al humano…",
	"Midiendo dos veces, cortando una…",
	"Esperando que el código se porte bien…",
	"Revisando si ya es viernes en UTC…",
	"Armando un plan que suene inteligente…",
	"Persiguiendo un race condition…",
	"Releyendo la documentación que ignoramos…",
	"Convenciendo a Git de cooperar…",
	"Tomando aire antes del refactor…",
] as const

export const TOOL_ACTIVE_PHRASES = [
	"Ejecutando herramientas en segundo plano…",
	"El agente está en modo manos a la obra…",
	"Esperando que la terminal responda…",
	"Leyendo archivos como si fueran novelas…",
	"Escribiendo código con guantes de seda…",
	"Una herramienta va, otra vuelve…",
	"Negociando con el filesystem…",
	"Persuadiendo a grep de cooperar…",
] as const

export function pickThinkingPhrase(previous?: string, phase?: TurnPhase): string {
	const source =
		phase === "tool_active" ? TOOL_ACTIVE_PHRASES : THINKING_PHRASES
	const pool = previous ? source.filter((phrase) => phrase !== previous) : source
	return pool[Math.floor(Math.random() * pool.length)] ?? source[0]
}