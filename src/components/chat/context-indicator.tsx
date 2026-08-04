import {
	Popover,
	PopoverAnchor,
	PopoverContent,
} from "@/components/ui/popover"
import { useHoverPopover } from "@/hooks/use-hover-popover"
import { cn } from "@/lib/utils"
import type { ContextUsage } from "@/stores/atoms"

interface ContextIndicatorProps {
	usage: ContextUsage | null
	className?: string
}

function formatTokens(value: number): string {
	if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`
	if (value >= 1_000) return `${(value / 1_000).toFixed(1)}k`
	return value.toLocaleString()
}

function strokeClassForRatio(ratio: number): string {
	if (ratio >= 0.92) return "text-red-400"
	if (ratio >= 0.75) return "text-amber-300"
	return "text-white/55"
}

function ContextRing({
	ratio,
	size,
	strokeClass,
}: {
	ratio: number
	size: "sm" | "lg"
	strokeClass: string
}) {
	const radius = size === "sm" ? 7 : 18
	const viewBox = size === "sm" ? 18 : 44
	const center = viewBox / 2
	const circumference = 2 * Math.PI * radius
	const offset = circumference * (1 - ratio)
	const svgClass = size === "sm" ? "size-4" : "size-11"

	return (
		<svg
			viewBox={`0 0 ${viewBox} ${viewBox}`}
			className={cn(svgClass, "-rotate-90")}
			aria-hidden
		>
			<circle
				cx={center}
				cy={center}
				r={radius}
				fill="none"
				className="stroke-white/15"
				strokeWidth="2"
			/>
			<circle
				cx={center}
				cy={center}
				r={radius}
				fill="none"
				className={cn(
					"transition-[stroke-dashoffset] duration-300",
					strokeClass,
				)}
				stroke="currentColor"
				strokeWidth="2"
				strokeLinecap="round"
				strokeDasharray={circumference}
				strokeDashoffset={offset}
			/>
		</svg>
	)
}

export function ContextIndicator({ usage, className }: ContextIndicatorProps) {
	const { open, setOpen, showPopover, scheduleClose } = useHoverPopover()

	const size = usage?.size ?? 0
	const used = usage?.used ?? 0
	const ratio = size > 0 ? Math.min(1, used / size) : 0
	const strokeClass = strokeClassForRatio(ratio)
	const percentLabel = size > 0 ? `${Math.round(ratio * 100)}%` : "—"

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverAnchor asChild>
				<div
					role="img"
					aria-label="Uso de contexto del modelo"
					onMouseEnter={showPopover}
					onMouseLeave={scheduleClose}
					onFocus={showPopover}
					onBlur={scheduleClose}
					tabIndex={0}
					className={cn(
						"inline-flex shrink-0 cursor-default items-center justify-center rounded-md p-1 text-white/70 transition-colors hover:bg-white/[0.08] hover:text-white/90 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-white/15",
						className,
					)}
				>
					<ContextRing ratio={ratio} size="sm" strokeClass={strokeClass} />
				</div>
			</PopoverAnchor>
			<PopoverContent
				align="start"
				sideOffset={6}
				className="w-56 p-3"
				onMouseEnter={showPopover}
				onMouseLeave={scheduleClose}
				onOpenAutoFocus={(event) => event.preventDefault()}
			>
				<p className="text-xs font-medium text-fg">Contexto del modelo</p>
				<div className="mt-3 flex items-center gap-3">
					<div className="relative flex shrink-0 items-center justify-center">
						<ContextRing ratio={ratio} size="lg" strokeClass={strokeClass} />
						<span className="absolute text-[10px] font-medium text-white/80">
							{percentLabel}
						</span>
					</div>
					<dl className="min-w-0 flex-1 space-y-1.5 text-xs">
						<div className="flex items-baseline justify-between gap-2">
							<dt className="text-white/50">Máximo</dt>
							<dd className="font-medium tabular-nums text-fg">
								{size > 0 ? `${formatTokens(size)} tokens` : "—"}
							</dd>
						</div>
						<div className="flex items-baseline justify-between gap-2">
							<dt className="text-white/50">Consumido</dt>
							<dd className="font-medium tabular-nums text-fg">
								{formatTokens(used)} tokens
							</dd>
						</div>
						{size > 0 ? (
							<div className="flex items-baseline justify-between gap-2">
								<dt className="text-white/50">Disponible</dt>
								<dd className="font-medium tabular-nums text-fg">
									{formatTokens(Math.max(0, size - used))} tokens
								</dd>
							</div>
						) : null}
					</dl>
				</div>
			</PopoverContent>
		</Popover>
	)
}
