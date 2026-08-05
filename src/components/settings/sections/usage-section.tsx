import { useMemo } from "react"
import { useAtomValue } from "jotai"
import { SectionHeader, SettingRow } from "@/components/settings/sections/section-ui"
import { estimateSampleCost, formatUsd } from "@/lib/cost-table"
import {
	activeSessionIdAtom,
	mcpSavingsAtom,
	toolOutputStatsAtom,
	usageHistoryBySessionAtom,
	type UsageSample,
} from "@/stores/atoms"

function Sparkline({
	samples,
	width = 280,
	height = 56,
}: {
	samples: UsageSample[]
	width?: number
	height?: number
}) {
	const points = useMemo(() => {
		if (samples.length === 0) return ""
		const max = Math.max(...samples.map((s) => s.used), 1)
		const step = width / Math.max(samples.length - 1, 1)
		return samples
			.map(
				(s, i) =>
					`${(i * step).toFixed(1)},${(
						height -
						4 -
						(s.used / max) * (height - 8)
					).toFixed(1)}`,
			)
			.join(" ")
	}, [samples, width, height])

	if (samples.length < 2) {
		return (
			<p className="text-[11px] text-muted">
				Collecting samples… send a prompt to see context growth.
			</p>
		)
	}
	return (
		<svg
			viewBox={`0 0 ${width} ${height}`}
			className="h-14 w-full max-w-xs"
			aria-label="Context usage sparkline"
		>
			<defs>
				<linearGradient id="spark-fill" x1="0" y1="0" x2="0" y2="1">
					<stop offset="0%" stopColor="var(--fg)" stopOpacity="0.25" />
					<stop offset="100%" stopColor="var(--fg)" stopOpacity="0" />
				</linearGradient>
			</defs>
			<line
				x1="0"
				y1={height - 2}
				x2={width}
				y2={height - 2}
				stroke="var(--border)"
				strokeWidth="1"
			/>
			<polyline
				points={`0,${height} ${points} ${width},${height}`}
				fill="url(#spark-fill)"
				stroke="none"
			/>
			<polyline
				points={points}
				fill="none"
				stroke="var(--fg)"
				strokeWidth="1.5"
				strokeLinejoin="round"
				strokeLinecap="round"
			/>
		</svg>
	)
}

function formatBytes(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
	return `${(bytes / (1024 * 1024)).toFixed(2)} MB`
}

function formatTokens(n: number): string {
	if (!n) return "0"
	if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
	return `${n}`
}

export function UsageSection() {
	const activeSessionId = useAtomValue(activeSessionIdAtom)
	const usageBySession = useAtomValue(usageHistoryBySessionAtom)
	const toolOutputStats = useAtomValue(toolOutputStatsAtom)
	const mcpSavings = useAtomValue(mcpSavingsAtom)

	const activeSamples = activeSessionId ? usageBySession[activeSessionId] ?? [] : []
	const allSamples = Object.values(usageBySession).flat()

	const totalTokens = useMemo(
		() => allSamples.reduce((n, s) => n + (s.used ?? 0), 0),
		[allSamples],
	)
	// Model per session is not tracked yet, so we use the default cost-table row.
	const estimatedCost = useMemo(
		() => estimateSampleCost(undefined, totalTokens),
		[totalTokens],
	)
	const estCostDisplay = formatUsd(estimatedCost)

	const loadedServers = Object.entries(mcpSavings.loadedServers).sort(
		(a, b) => b[1] - a[1],
	)
	const totalMcpLoads = loadedServers.reduce((n, [, count]) => n + count, 0)
	const lastSample = activeSamples[activeSamples.length - 1]

	return (
		<div>
			<SectionHeader
				title="Usage"
				description="Token tracking from usage_update, tool output volume and measured savings. All costs are estimates."
			/>
			<div className="space-y-3">
				<SettingRow
					label="Context usage"
					description={
						lastSample
							? `${formatTokens(lastSample.used)} / ${formatTokens(lastSample.size)} tokens used in the active session`
							: "Samples arrive as the agent streams usage_update events."
					}
					control={
						<div className="flex min-w-0 flex-col items-end gap-1.5">
							<Sparkline samples={activeSamples} />
						</div>
					}
				/>
				<SettingRow
					label="Sessions tracked"
					description="Samples from every session in this run, kept in memory."
					control={
						<span className="text-sm tabular-nums text-fg">
							{Object.keys(usageBySession).length}
						</span>
					}
				/>
				<SettingRow
					label="Total tokens"
					description="Sum of usage_update samples across tracked sessions."
					control={
						<span className="text-sm tabular-nums text-fg">
							{totalTokens.toLocaleString()}
						</span>
					}
				/>
				<SettingRow
					label="Estimated cost"
					description="From src/lib/cost-table.ts with the default row — model-specific pricing is not tracked per session yet."
					control={
						<span className="text-sm tabular-nums text-fg">{estCostDisplay}</span>
					}
				/>
				<SettingRow
					label="Tool output measured"
					description={`${toolOutputStats.toolCallCount} tool calls streamed — ${formatBytes(toolOutputStats.totalOutputBytes)} of content passed through the transcript.`}
					control={
						<span className="text-sm tabular-nums text-fg">
							{formatBytes(toolOutputStats.totalOutputBytes)}
						</span>
					}
				/>
				<SettingRow
					label="Savings from compact_result"
					description={`${mcpSavings.compactionCount} compactions measured — bytes the optimizer removed from tool results (terminal filters arrive in a later phase).`}
					control={
						<span className="text-sm tabular-nums text-fg">
							{formatBytes(mcpSavings.savingsBytes)}
						</span>
					}
				/>
				<SettingRow
					label="MCP servers loaded"
					description={
						totalMcpLoads === 0
							? "Use /mcp <name> or ask the agent to call mcp_load to see usage here."
							: `${totalMcpLoads} loads across ${loadedServers.length} servers.`
					}
					control={
						loadedServers.length === 0 ? (
							<span className="text-sm text-muted">—</span>
						) : (
							<div className="flex max-w-56 flex-col items-end gap-1">
								{loadedServers.map(([name, count]) => (
									<span
										key={name}
										className="rounded bg-white/5 px-1.5 py-0.5 font-mono text-[10px] text-fg/80"
									>
										{name} ×{count}
									</span>
								))}
							</div>
						)
					}
				/>
			</div>
		</div>
	)
}
