import { X } from "lucide-react"
import { useEffect, type ReactNode } from "react"
import { createPortal } from "react-dom"
import { windowNoDragProps } from "@/hooks/use-window-drag"
import { cn } from "@/lib/utils"

interface OverlayShellProps {
	open: boolean
	title: string
	subtitle?: string
	badge?: string
	onClose: () => void
	children: ReactNode
	footer?: ReactNode
	className?: string
}

export function OverlayShell({
	open,
	title,
	subtitle,
	badge,
	onClose,
	children,
	footer,
	className,
}: OverlayShellProps) {
	const noDragProps = windowNoDragProps()

	useEffect(() => {
		if (!open) return

		function onKeyDown(event: KeyboardEvent) {
			if (event.key === "Escape") {
				event.preventDefault()
				onClose()
			}
		}

		const previousOverflow = document.body.style.overflow
		document.body.style.overflow = "hidden"
		window.addEventListener("keydown", onKeyDown)

		return () => {
			document.body.style.overflow = previousOverflow
			window.removeEventListener("keydown", onKeyDown)
		}
	}, [open, onClose])

	if (!open) return null

	return createPortal(
		<div
			className="fixed inset-0 z-[120] flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm"
			{...noDragProps}
			onClick={onClose}
		>
			<div
				role="dialog"
				aria-modal="true"
				aria-label={title}
				className={cn(
					"flex max-h-[min(92vh,900px)] w-full max-w-5xl flex-col overflow-hidden rounded-xl border border-border bg-card shadow-2xl",
					className,
				)}
				onClick={(event) => event.stopPropagation()}
			>
				<div className="flex items-start gap-3 border-b border-border/60 px-4 py-3">
					<div className="min-w-0 flex-1">
						<div className="flex flex-wrap items-center gap-2">
							<h2 className="truncate text-sm font-medium text-foreground">{title}</h2>
							{badge ? (
								<span className="rounded-md border border-border px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-muted-foreground">
									{badge}
								</span>
							) : null}
						</div>
						{subtitle ? (
							<p className="mt-0.5 truncate font-mono text-xs text-muted-foreground">
								{subtitle}
							</p>
						) : null}
					</div>
					<button
						type="button"
						onClick={onClose}
						className="rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
						aria-label="Cerrar"
					>
						<X className="size-4" />
					</button>
				</div>

				<div className="scrollbar-thin min-h-0 flex-1 overflow-auto">{children}</div>

				{footer ? (
					<div className="border-t border-border/60 px-4 py-3">{footer}</div>
				) : null}
			</div>
		</div>,
		document.body,
	)
}