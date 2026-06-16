import { useAtomValue } from "jotai"
import { useEffect } from "react"
import { hasWaitingAtom } from "../atoms/derived/waiting"

/**
 * Updates the browser tab title when any agent is waiting for user input.
 */
export function useWaitingIndicator() {
	const hasWaiting = useAtomValue(hasWaitingAtom)

	useEffect(() => {
		document.title = hasWaiting ? "(!) Circulo \u2014 Input needed" : "Circulo"

		return () => {
			document.title = "Circulo"
		}
	}, [hasWaiting])
}
