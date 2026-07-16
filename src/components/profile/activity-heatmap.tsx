import { type CSSProperties, useMemo } from "react"
import {
	formatCompact,
	formatShortDate,
	type ProfileHeatmapCell,
} from "@/lib/profile-activity"
import { cn } from "@/lib/utils"

export const HEATMAP_INTENSITY_CLASSES: readonly string[] = [
	"bg-muted/70 dark:bg-white/[0.06]",
	"bg-[color-mix(in_srgb,var(--info)_24%,transparent)]",
	"bg-[color-mix(in_srgb,var(--info)_46%,transparent)]",
	"bg-[color-mix(in_srgb,var(--info)_72%,transparent)]",
	"bg-[var(--info)]",
]

const MONTH_LABELS = [
	"Jan",
	"Feb",
	"Mar",
	"Apr",
	"May",
	"Jun",
	"Jul",
	"Aug",
	"Sep",
	"Oct",
	"Nov",
	"Dec",
]

interface ActivityHeatmapProps {
	cells: ReadonlyArray<ProfileHeatmapCell>
	gap?: number
	radius?: number
	showMonths?: boolean
	monthsPosition?: "top" | "bottom"
	tooltipUnit?: string
	className?: string
}

type Slot =
	| { kind: "cell"; cell: ProfileHeatmapCell }
	| { kind: "pad"; id: string }

interface Column {
	key: string
	slots: ReadonlyArray<Slot>
}

function heatmapTooltipText(cell: ProfileHeatmapCell, unit: string): string {
	const date = formatShortDate(cell.day) ?? cell.day
	if (cell.count <= 0) {
		return `No ${unit} on ${date}`
	}
	const noun = cell.count === 1 && unit.endsWith("s") ? unit.slice(0, -1) : unit
	return `${formatCompact(cell.count)} ${noun} on ${date}`
}

export function ActivityHeatmap({
	cells,
	gap = 3,
	radius = 5,
	showMonths = true,
	monthsPosition = "bottom",
	tooltipUnit = "prompts",
	className,
}: ActivityHeatmapProps) {
	const columns = useMemo<Column[]>(() => {
		if (cells.length === 0) return []

		const slots: Slot[] = []
		for (let index = 0; index < cells[0]!.weekday; index += 1) {
			slots.push({ kind: "pad", id: `pad-lead-${index}` })
		}
		for (const cell of cells) {
			slots.push({ kind: "cell", cell })
		}
		while (slots.length % 7 !== 0) {
			slots.push({ kind: "pad", id: `pad-tail-${slots.length}` })
		}

		const result: Column[] = []
		for (let index = 0; index < slots.length; index += 7) {
			const week = slots.slice(index, index + 7)
			const firstCell = week.find(
				(slot): slot is Extract<Slot, { kind: "cell" }> => slot.kind === "cell",
			)
			result.push({ key: firstCell ? firstCell.cell.day : `col-${index}`, slots: week })
		}
		return result
	}, [cells])

	const monthByColumn = useMemo<(string | null)[]>(() => {
		let previousMonth = -1
		return columns.map((column) => {
			const firstCell = column.slots.find(
				(slot): slot is Extract<Slot, { kind: "cell" }> => slot.kind === "cell",
			)
			if (!firstCell) return null
			const monthIndex = Number(firstCell.cell.day.split("-")[1]) - 1
			if (monthIndex === previousMonth || monthIndex < 0) return null
			previousMonth = monthIndex
			return MONTH_LABELS[monthIndex] ?? null
		})
	}, [columns])

	const columnStyle: CSSProperties = { gap: `${gap}px` }
	const cellStyle: CSSProperties = { borderRadius: `${radius}px` }

	const monthRow = showMonths ? (
		<div className="flex w-full min-w-0" style={columnStyle}>
			{columns.map((column, index) => (
				<div
					key={`month-${column.key}`}
					className="min-w-0 flex-1 overflow-visible whitespace-nowrap text-[10px] font-medium leading-none text-muted-foreground"
				>
					{monthByColumn[index] ?? ""}
				</div>
			))}
		</div>
	) : null

	if (cells.length === 0) {
		return (
			<p className="rounded-lg border border-dashed border-border/60 px-4 py-8 text-center text-sm text-muted-foreground">
				No activity recorded yet. Send a prompt to start your heatmap.
			</p>
		)
	}

	return (
		<div className={cn("flex w-full min-w-0 flex-col", className)} style={{ gap: `${gap}px` }}>
			{showMonths && monthsPosition === "top" ? monthRow : null}
			<div className="flex w-full min-w-0" style={columnStyle}>
				{columns.map((column) => (
					<div
						key={column.key}
						className="flex min-w-0 flex-1 flex-col"
						style={columnStyle}
					>
						{column.slots.map((slot) => {
							if (slot.kind !== "cell") {
								return (
									<div
										key={slot.id}
										className="aspect-square w-full min-w-0 bg-transparent"
										style={cellStyle}
									/>
								)
							}
							return (
								<div
									key={slot.cell.day}
									className={cn(
										"aspect-square w-full min-w-0 transition-colors",
										HEATMAP_INTENSITY_CLASSES[slot.cell.intensity] ??
											HEATMAP_INTENSITY_CLASSES[0],
									)}
									style={cellStyle}
									title={heatmapTooltipText(slot.cell, tooltipUnit)}
								/>
							)
						})}
					</div>
				))}
			</div>
			{showMonths && monthsPosition === "bottom" ? monthRow : null}
		</div>
	)
}