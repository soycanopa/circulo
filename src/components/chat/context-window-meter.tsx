import { useRef, useState } from "react"
import { SelectorPortalMenu } from "@/components/chat/selector-portal-menu"
import {
	formatContextTokens,
	formatCostUsd,
	deriveMeterPercent,
	type ContextWindowSnapshot,
} from "@/lib/context-window"
import { cn } from "@/lib/utils"

interface ContextWindowMeterProps {
	usage: ContextWindowSnapshot | null
}

function formatUsageHeader(usage: ContextWindowSnapshot): string {
	const used = formatContextTokens(usage.usedTokens)
	if (usage.maxTokens) {
		const max = formatContextTokens(usage.maxTokens)
		const percent =
			usage.usedPercent !== null
				? usage.usedPercent < 10
					? `${usage.usedPercent.toFixed(1).replace(/\.0$/, "")}%`
					: `${Math.round(usage.usedPercent)}%`
				: null
		return percent ? `${used}/${max} (${percent})` : `${used}/${max}`
	}
	return `${used} tokens`
}

export function ContextWindowMeter({ usage }: ContextWindowMeterProps) {
	const [open, setOpen] = useState(false)
	const triggerRef = useRef<HTMLButtonElement>(null)
	const percent = usage ? deriveMeterPercent(usage) : 0
	const radius = 6
	const circumference = 2 * Math.PI * radius
	const dashOffset = circumference - (percent / 100) * circumference
	const hasData = usage !== null && usage.usedTokens > 0

	return (
		<>
			<button
				ref={triggerRef}
				type="button"
				onClick={() => setOpen((current) => !current)}
				className={cn(
					"inline-flex shrink-0 items-center justify-center rounded-full p-0.5 transition-opacity hover:opacity-80",
					!hasData && "opacity-50",
				)}
				aria-label={
					usage
						? `Ventana de contexto: ${formatUsageHeader(usage)}`
						: "Ventana de contexto: sin datos"
				}
			>
				<span className="relative flex size-4 items-center justify-center">
					<svg
						viewBox="0 0 16 16"
						className="absolute inset-0 size-full -rotate-90 transform-gpu"
						aria-hidden="true"
					>
						<circle
							cx="8"
							cy="8"
							r={radius}
							fill="none"
							stroke="currentColor"
							strokeWidth="2"
							className="text-muted-foreground/30"
						/>
						<circle
							cx="8"
							cy="8"
							r={radius}
							fill="none"
							stroke="currentColor"
							strokeWidth="2"
							strokeLinecap="round"
							strokeDasharray={circumference}
							strokeDashoffset={hasData ? dashOffset : circumference}
							className="text-[#3B5EF9] transition-[stroke-dashoffset] duration-500 ease-out"
						/>
					</svg>
				</span>
			</button>

			<SelectorPortalMenu
				open={open}
				anchorRef={triggerRef}
				onClose={() => setOpen(false)}
				className="w-72 p-3"
				minWidth={288}
			>
				<div className="space-y-3">
					<div className="flex items-center justify-between gap-3">
						<p className="text-sm font-medium text-foreground">Context windows</p>
						<p className="text-xs text-muted-foreground">
							{usage ? formatUsageHeader(usage) : "—"}
						</p>
					</div>

					<div className="h-1.5 overflow-hidden rounded-full bg-muted/60">
						<div
							className="h-full rounded-full bg-[#3B5EF9] transition-[width] duration-500 ease-out"
							style={{ width: `${hasData ? percent : 0}%` }}
						/>
					</div>

					{usage?.breakdown.length ? (
						<ul className="space-y-1.5">
							{usage.breakdown.map((item) => (
								<li
									key={item.id}
									className="flex items-center justify-between gap-3 text-xs"
								>
									<span className="flex min-w-0 items-center gap-2 text-foreground/90">
										<span className="size-1.5 shrink-0 rounded-full bg-[#3B5EF9]" />
										<span className="truncate">{item.label}</span>
									</span>
									<span className="shrink-0 tabular-nums text-muted-foreground">
										{item.percent < 10
											? `${item.percent.toFixed(1).replace(/\.0$/, "")}%`
											: `${Math.round(item.percent)}%`}
									</span>
								</li>
							))}
						</ul>
					) : (
						<p className="text-xs leading-relaxed text-muted-foreground">
							{usage
								? "El desglose por categoría aparecerá cuando el agente lo reporte. Uso total arriba."
								: "Aún no hay datos de contexto para esta sesión."}
						</p>
					)}

					{usage?.costUsd != null ? (
						<p className="border-t border-border/50 pt-2 text-xs text-muted-foreground">
							Coste de sesión: {formatCostUsd(usage.costUsd)}
						</p>
					) : null}
				</div>
			</SelectorPortalMenu>
		</>
	)
}