import { FileDiff } from "lucide-react"
import type { ButtonHTMLAttributes } from "react"
import { DiffStatLabel } from "@/components/chat/diff-stat-label"
import { hasDiffStats, type DiffLineStats } from "@/lib/session-diff-stats"
import { chromeIconButtonActiveClass, chromeIconButtonClass } from "@/lib/control-button"
import { cn } from "@/lib/utils"

interface DiffPanelOpenButtonProps
	extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
	ariaLabel?: string
	active?: boolean
	activeClassName?: string
	stats?: DiffLineStats
}

export function DiffPanelOpenButton({
	onClick,
	title = "Abrir panel de cambios (⌘⇧D)",
	ariaLabel = "Abrir panel de cambios",
	active = false,
	activeClassName,
	stats,
	className,
	...props
}: DiffPanelOpenButtonProps) {
	const hasStats = stats ? hasDiffStats(stats) : false

	return (
		<button
			type="button"
			{...props}
			onClick={onClick}
			title={title}
			aria-label={ariaLabel}
			aria-pressed={active}
			className={cn(
				chromeIconButtonClass,
				hasStats ? "h-7 gap-1.5 pl-2 pr-1.5 text-[11px] font-normal" : "size-7",
				active && (activeClassName ?? chromeIconButtonActiveClass),
				className,
			)}
		>
			{hasStats && stats ? (
				<DiffStatLabel
					additions={stats.additions}
					deletions={stats.deletions}
					className="tabular-nums"
				/>
			) : null}
			<FileDiff className="size-3.5 shrink-0" aria-hidden />
		</button>
	)
}