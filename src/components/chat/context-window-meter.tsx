import { useRef, useState } from "react"
import { SelectorPortalMenu } from "@/components/chat/selector-portal-menu"
import {
	formatContextTokens,
	formatCostUsd,
	deriveMeterPercent,
	hasContextWindowData,
	type ContextWindowSnapshot,
} from "@/lib/context-window"
import { cn } from "@/lib/utils"

interface ContextWindowMeterProps {
	usage: ContextWindowSnapshot | null
}

function formatUsageCompact(usage: ContextWindowSnapshot): string {
	if (usage.maxTokens) {
		const used = formatContextTokens(usage.usedTokens)
		const max = formatContextTokens(usage.maxTokens)
		if (usage.usedPercent !== null) {
			const pct =
				usage.usedPercent < 10
					? usage.usedPercent.toFixed(1).replace(/\.0$/, "")
					: `${Math.round(usage.usedPercent)}`
			return `${used}/${max} · ${pct}%`
		}
		return `${used}/${max}`
	}
	return formatContextTokens(usage.usedTokens)
}

function ContextFillBar({
	percent,
	className,
	barClassName,
}: {
	percent: number
	className?: string
	barClassName?: string
}) {
	const clamped = Math.max(0, Math.min(100, percent))
	const showFill = clamped > 0

	return (
		<div
			className={cn(
				"relative w-full overflow-hidden rounded-full bg-[#414141]",
				className,
			)}
			role="progressbar"
			aria-valuenow={clamped}
			aria-valuemin={0}
			aria-valuemax={100}
		>
			{showFill ? (
				<div
					className={cn(
						"absolute inset-y-0 left-0 min-w-[3px] rounded-full bg-[#3B5EF9] transition-[width] duration-500 ease-out",
						barClassName,
					)}
					style={{ width: `${clamped}%` }}
				/>
			) : null}
		</div>
	)
}

export function ContextWindowMeter({ usage }: ContextWindowMeterProps) {
	const [open, setOpen] = useState(false)
	const triggerRef = useRef<HTMLButtonElement>(null)
	const percent = usage ? deriveMeterPercent(usage) : 0
	const radius = 6
	const circumference = 2 * Math.PI * radius
	const dashOffset = circumference - (percent / 100) * circumference
	const hasData = hasContextWindowData(usage)

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
						? `Contexto: ${formatUsageCompact(usage)}`
						: "Contexto: sin datos"
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
				preferPlacement="above"
				className="w-[min(14rem,calc(100vw-1rem))] p-2.5"
				minWidth={200}
			>
				<div className="space-y-2.5">
					<div className="flex items-center justify-between gap-2">
						<p className="text-sm font-medium text-foreground">Contexto</p>
						<p className="shrink-0 text-xs tabular-nums text-muted-foreground">
							{usage ? formatUsageCompact(usage) : "—"}
						</p>
					</div>

					<ContextFillBar percent={hasData ? percent : 0} className="h-2.5" />

					{usage?.breakdown.length ? (
						<ul className="space-y-2">
							{usage.breakdown.map((item) => (
								<li key={item.id}>
									<div className="mb-1 flex items-center justify-between gap-2 text-xs text-muted-foreground">
										<span className="truncate">{item.label}</span>
										<span className="shrink-0 tabular-nums">
											{item.percent < 10
												? `${item.percent.toFixed(1).replace(/\.0$/, "")}%`
												: `${Math.round(item.percent)}%`}
										</span>
									</div>
									<ContextFillBar
										percent={item.percent}
										className="h-1.5"
										barClassName="bg-[#3B5EF9]/85"
									/>
								</li>
							))}
						</ul>
					) : null}

					{usage?.costUsd != null ? (
						<p className="text-xs tabular-nums text-muted-foreground">
							{formatCostUsd(usage.costUsd)}
						</p>
					) : null}
				</div>
			</SelectorPortalMenu>
		</>
	)
}