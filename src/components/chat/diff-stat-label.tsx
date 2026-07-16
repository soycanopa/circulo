import { hasDiffStats, type DiffLineStats } from "@/lib/session-diff-stats"
import { cn } from "@/lib/utils"

interface DiffStatLabelProps extends DiffLineStats {
	className?: string
}

function DiffStatValues({ additions, deletions }: DiffLineStats) {
	return (
		<span className="inline-flex items-baseline gap-1.5 tabular-nums">
			<span className="text-diff-addition">+{additions}</span>
			<span className="text-diff-deletion">-{deletions}</span>
		</span>
	)
}

export function DiffStatLabel({ additions, deletions, className }: DiffStatLabelProps) {
	if (!hasDiffStats({ additions, deletions })) return null

	return (
		<span className={cn(className)}>
			<DiffStatValues additions={additions} deletions={deletions} />
		</span>
	)
}