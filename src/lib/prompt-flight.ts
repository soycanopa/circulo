/** Synchronous prompt-flight flag — avoids React state lag before ACP chunks arrive. */
export const promptInFlightRef = { current: false }

export function setPromptInFlightSync(value: boolean): void {
	promptInFlightRef.current = value
}