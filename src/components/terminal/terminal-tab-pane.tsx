import { useEffect, useCallback, useState } from "react"
import { useTerminal } from "@/hooks/use-terminal"
import { cn } from "@/lib/utils"

interface TerminalTabPaneProps {
	tabId: string
	active: boolean
	cwd: string | null
	onRegister: (tabId: string, api: { fit: () => void; focus: () => void }) => void
}

export function TerminalTabPane({ tabId, active, cwd, onRegister }: TerminalTabPaneProps) {
	const [container, setContainer] = useState<HTMLDivElement | null>(null)
	const setContainerRef = useCallback((node: HTMLDivElement | null) => {
		setContainer(node)
	}, [])

	const { fit, focus } = useTerminal({
		container,
		cwd,
	})

	useEffect(() => {
		onRegister(tabId, { fit, focus })
	}, [tabId, fit, focus, onRegister])

	useEffect(() => {
		if (!active) return
		const frame = requestAnimationFrame(() => {
			fit()
			focus()
		})
		return () => cancelAnimationFrame(frame)
	}, [active, fit, focus])

	return (
		<div
			ref={setContainerRef}
			data-slot="terminal-viewport"
			aria-hidden={!active}
			className={cn(
				"absolute inset-0 overflow-hidden",
				!active && "pointer-events-none invisible",
			)}
		/>
	)
}