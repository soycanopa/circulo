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

export function pickThinkingPhrase(previous?: string): string {
	const pool = previous
		? THINKING_PHRASES.filter((phrase) => phrase !== previous)
		: THINKING_PHRASES
	return pool[Math.floor(Math.random() * pool.length)] ?? THINKING_PHRASES[0]
}