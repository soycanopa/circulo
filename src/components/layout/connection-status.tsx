import { useAtomValue } from "jotai"
import { Monitor } from "lucide-react"
import { useLayoutEffect, useRef, useState } from "react"
import { createPortal } from "react-dom"
import { getProjectDisplayName } from "@/lib/project-display"
import { cn } from "@/lib/utils"
import {
	errorMessageAtom,
	projectPathAtom,
	sessionStatusAtom,
} from "@/stores/atoms"
import type { SessionStatus } from "@/types/acp"

interface ConnectionStatusProps {
	connected: boolean
}

function statusColor(connected: boolean, sessionStatus: SessionStatus): string {
	if (!connected || sessionStatus === "disconnected") return "bg-red-500"
	if (
		sessionStatus === "awaiting_permission" ||
		sessionStatus === "awaiting_credential" ||
		sessionStatus === "generating"
	) {
		return "bg-amber-400"
	}
	return "bg-green-500"
}

function statusLabel(connected: boolean, sessionStatus: SessionStatus): string {
	if (!connected || sessionStatus === "disconnected") return "Desconectado"
	if (sessionStatus === "awaiting_permission") return "Esperando permiso"
	if (sessionStatus === "awaiting_credential") return "Esperando credenciales"
	if (sessionStatus === "generating") return "Generando respuesta"
	return "Conectado"
}

export function ConnectionStatus({ connected }: ConnectionStatusProps) {
	const sessionStatus = useAtomValue(sessionStatusAtom)
	const projectPath = useAtomValue(projectPathAtom)
	const errorMessage = useAtomValue(errorMessageAtom)
	const [open, setOpen] = useState(false)
	const [popoverStyle, setPopoverStyle] = useState<React.CSSProperties>({})
	const anchorRef = useRef<HTMLButtonElement>(null)
	const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

	function updatePosition() {
		const anchor = anchorRef.current
		if (!anchor) return
		const rect = anchor.getBoundingClientRect()
		setPopoverStyle({
			position: "fixed",
			left: rect.left,
			bottom: window.innerHeight - rect.top + 8,
			zIndex: 200,
		})
	}

	function handleEnter() {
		if (closeTimer.current) clearTimeout(closeTimer.current)
		setOpen(true)
	}

	function handleLeave() {
		closeTimer.current = setTimeout(() => setOpen(false), 120)
	}

	useLayoutEffect(() => {
		if (!open) return
		updatePosition()
		window.addEventListener("resize", updatePosition)
		window.addEventListener("scroll", updatePosition, true)
		return () => {
			window.removeEventListener("resize", updatePosition)
			window.removeEventListener("scroll", updatePosition, true)
		}
	}, [open])

	const color = statusColor(connected, sessionStatus)
	const label = statusLabel(connected, sessionStatus)
	const projectName = getProjectDisplayName(projectPath)

	return (
		<>
			<div className="px-2 py-1" onMouseEnter={handleEnter} onMouseLeave={handleLeave}>
				<button
					ref={anchorRef}
					type="button"
					className="relative flex size-8 items-center justify-center rounded-md text-sidebar-foreground/70 transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
					aria-label={label}
				>
					<Monitor className="size-4" />
					<span
						className={cn(
							"absolute bottom-1.5 right-1.5 size-2 rounded-full ring-2 ring-sidebar",
							color,
						)}
					/>
				</button>
			</div>

			{open
				? createPortal(
						<div
							style={popoverStyle}
							className="w-56 rounded-lg border border-border bg-popover p-3 text-xs shadow-lg"
							onMouseEnter={handleEnter}
							onMouseLeave={handleLeave}
						>
							<p className="font-medium text-popover-foreground">{label}</p>
							<dl className="mt-2 space-y-1.5 text-muted-foreground">
								<div className="flex justify-between gap-3">
									<dt>Agente</dt>
									<dd className="text-right text-popover-foreground">opencode acp</dd>
								</div>
								<div className="flex justify-between gap-3">
									<dt>Estado</dt>
									<dd className="text-right text-popover-foreground">{sessionStatus}</dd>
								</div>
								{projectPath ? (
									<div className="flex justify-between gap-3">
										<dt>Proyecto</dt>
										<dd className="max-w-[9rem] truncate text-right text-popover-foreground">
											{projectName}
										</dd>
									</div>
								) : null}
								{errorMessage ? (
									<div>
										<dt className="mb-0.5">Último error</dt>
										<dd className="text-destructive">{errorMessage}</dd>
									</div>
								) : null}
							</dl>
						</div>,
						document.body,
					)
				: null}
		</>
	)
}