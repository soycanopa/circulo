import { Pencil } from "lucide-react"
import { useAtomValue } from "jotai"
import { useEffect, useMemo, useState } from "react"
import { ActivityHeatmap } from "@/components/profile/activity-heatmap"
import { EditProfileDialog } from "@/components/profile/edit-profile-dialog"
import { ProfileAvatar } from "@/components/profile/profile-avatar"
import { getArchivedSessionIds } from "@/lib/archived-sessions"
import {
	formatContextTokens,
	formatCostUsd,
	hasContextWindowData,
} from "@/lib/context-window"
import {
	formatCompact,
	formatDays,
	getProfileActivitySummary,
	selectProfileHeatmap,
	seedProfileActivityFromSessions,
} from "@/lib/profile-activity"
import { getFavoriteModels } from "@/lib/favorite-models"
import { getLastModel } from "@/lib/preferences"
import { getPinnedSessionIds } from "@/lib/pinned-sessions"
import {
	getDefaultProfileHandle,
	getDefaultProfileName,
	getProfileInitials,
	useProfileAvatarColor,
	useProfileAvatarImage,
	useProfileHandle,
	useProfileName,
} from "@/lib/profile-identity"
import { getProjectDisplayName } from "@/lib/project-display"
import { getRecentProjects } from "@/lib/recent-projects"
import { sessionTitle } from "@/lib/sessions"
import type { SessionInfo } from "@/types/acp"
import {
	activeSessionIdAtom,
	contextWindowAtom,
	projectPathAtom,
	sessionsAtom,
} from "@/stores/atoms"

function sessionSortTime(session: SessionInfo): number {
	if (!session.updatedAt) return 0
	const parsed = Date.parse(session.updatedAt)
	return Number.isNaN(parsed) ? 0 : parsed
}

function StatTile({ label, value }: { label: string; value: string }) {
	return (
		<div className="flex flex-col items-center gap-0.5 px-3 py-3">
			<span className="text-sm font-normal tabular-nums text-foreground">{value}</span>
			<span className="text-sm font-normal text-muted-foreground">{label}</span>
		</div>
	)
}

function InsightRow({ label, value }: { label: string; value: string }) {
	return (
		<div className="flex items-center justify-between gap-3">
			<dt className="shrink-0 text-sm text-muted-foreground">{label}</dt>
			<dd className="truncate text-sm font-normal tabular-nums" title={value}>
				{value}
			</dd>
		</div>
	)
}

export function ProfileSettings() {
	const sessions = useAtomValue(sessionsAtom)
	const activeSessionId = useAtomValue(activeSessionIdAtom)
	const projectPath = useAtomValue(projectPathAtom)
	const contextWindow = useAtomValue(contextWindowAtom)
	const [editOpen, setEditOpen] = useState(false)
	const [activityVersion, setActivityVersion] = useState(0)

	const defaultName = getDefaultProfileName()
	const defaultHandle = getDefaultProfileHandle()
	const { name, setName } = useProfileName(defaultName)
	const { handle, setHandle } = useProfileHandle(defaultHandle)
	const { color: avatarColor, setColor: setAvatarColor } = useProfileAvatarColor()
	const { image: avatarImage, setImage: setAvatarImage } = useProfileAvatarImage()

	useEffect(() => {
		seedProfileActivityFromSessions(sessions)
	}, [sessions])

	useEffect(() => {
		function refresh() {
			setActivityVersion((value) => value + 1)
		}
		window.addEventListener("circulo:profile-activity-changed", refresh)
		return () => window.removeEventListener("circulo:profile-activity-changed", refresh)
	}, [])

	const activity = useMemo(() => getProfileActivitySummary(), [activityVersion])
	const heatmap = useMemo(() => selectProfileHeatmap(), [activityVersion])

	const activeIndex = sessions.findIndex((session) => session.sessionId === activeSessionId)
	const activeSession = activeIndex >= 0 ? sessions[activeIndex] : null

	const mostWorkedProject = useMemo(() => {
		const counts = new Map<string, number>()
		for (const session of sessions) {
			const key = session.cwd || "unknown"
			counts.set(key, (counts.get(key) ?? 0) + 1)
		}
		let bestPath: string | null = null
		let bestCount = 0
		for (const [path, count] of counts) {
			if (count > bestCount) {
				bestPath = path
				bestCount = count
			}
		}
		if (!bestPath) return null
		return { label: getProjectDisplayName(bestPath), count: bestCount }
	}, [sessions])

	const recentSessions = useMemo(
		() =>
			[...sessions]
				.sort((a, b) => sessionSortTime(b) - sessionSortTime(a))
				.slice(0, 5),
		[sessions],
	)

	const archivedCount = getArchivedSessionIds().length
	const pinnedCount = getPinnedSessionIds().length
	const favoriteCount = getFavoriteModels().length
	const projectCount = getRecentProjects().length
	const initials = getProfileInitials(name)

	const lifetimeTokens =
		activity.lifetimeTokens > 0
			? formatCompact(activity.lifetimeTokens)
			: hasContextWindowData(contextWindow)
				? formatContextTokens(contextWindow?.usedTokens)
				: "—"

	return (
		<div className="flex min-w-0 flex-col gap-7">
			<div className="flex items-center justify-end gap-2">
				<button
					type="button"
					onClick={() => setEditOpen(true)}
					className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border/60 px-3 text-xs text-foreground transition-colors hover:bg-accent"
				>
					<Pencil className="size-3.5" />
					Edit
				</button>
			</div>

			<header className="flex flex-col items-center gap-3 text-center">
				<ProfileAvatar
					initials={initials}
					color={avatarColor}
					image={avatarImage}
					className="size-16 shadow-sm"
					textClassName="text-xl"
				/>
				<div className="flex flex-col items-center gap-1.5">
					<h2 className="text-2xl font-semibold tracking-tight">{name}</h2>
					<div className="flex items-center gap-1.5 text-sm text-muted-foreground">
						<span>{handle}</span>
						<span aria-hidden>·</span>
						<span className="rounded-full border border-border/60 px-1.5 py-px text-xs text-muted-foreground">
							Circulo
						</span>
					</div>
				</div>
			</header>

			<div className="grid grid-cols-2 divide-x divide-y divide-border/50 overflow-hidden rounded-2xl border border-border/60 sm:grid-cols-3 lg:grid-cols-5 lg:divide-y-0">
				<StatTile label="Lifetime tokens" value={lifetimeTokens} />
				<StatTile
					label="Peak day"
					value={
						activity.peakDayTokens > 0
							? formatCompact(activity.peakDayTokens)
							: "—"
					}
				/>
				<StatTile label="Total prompts" value={formatCompact(activity.totalPrompts)} />
				<StatTile label="Current streak" value={formatDays(activity.currentStreakDays)} />
				<StatTile label="Longest streak" value={formatDays(activity.longestStreakDays)} />
			</div>

			<section className="flex min-w-0 flex-col gap-3">
				<div className="flex items-center justify-between gap-3">
					<h3 className="text-sm font-medium">Activity</h3>
					<span className="text-[10px] uppercase tracking-wide text-muted-foreground">
						{heatmap.unit}
					</span>
				</div>
				<ActivityHeatmap
					cells={heatmap.cells}
					tooltipUnit={heatmap.unit}
					showMonths
					monthsPosition="bottom"
				/>
				<div className="flex items-center justify-end gap-1.5 text-[10px] text-muted-foreground">
					<span>Less</span>
					{[0, 1, 2, 3, 4].map((level) => (
						<span
							key={level}
							className="size-2.5 rounded-[3px]"
							style={{
								backgroundColor:
									level === 0
										? "color-mix(in srgb, var(--muted) 70%, transparent)"
										: `color-mix(in srgb, var(--info) ${level === 1 ? 24 : level === 2 ? 46 : level === 3 ? 72 : 100}%, transparent)`,
							}}
						/>
					))}
					<span>More</span>
				</div>
			</section>

			<div className="grid gap-x-12 gap-y-7 md:grid-cols-2">
				<section className="flex flex-col gap-3">
					<h3 className="text-sm font-medium">Activity insights</h3>
					<dl className="flex flex-col gap-2.5">
						<InsightRow
							label="Most worked project"
							value={
								mostWorkedProject
									? `${mostWorkedProject.label} · ${mostWorkedProject.count} sessions`
									: "—"
							}
						/>
						<InsightRow
							label="Active project"
							value={getProjectDisplayName(projectPath)}
						/>
						<InsightRow
							label="Last model"
							value={getLastModel() ?? "—"}
						/>
						<InsightRow label="Saved projects" value={String(projectCount)} />
						<InsightRow label="Pinned sessions" value={String(pinnedCount)} />
						<InsightRow label="Archived sessions" value={String(archivedCount)} />
						<InsightRow label="Favorite models" value={String(favoriteCount)} />
						<InsightRow label="Total sessions" value={String(sessions.length)} />
					</dl>
				</section>

				<section className="flex flex-col gap-3">
					<h3 className="text-sm font-medium">Active session</h3>
					<dl className="flex flex-col gap-2.5">
						<InsightRow
							label="Session"
							value={
								activeSession
									? sessionTitle(activeSession, activeIndex)
									: "Sin sesión activa"
							}
						/>
						<InsightRow
							label="Context used"
							value={
								hasContextWindowData(contextWindow)
									? formatContextTokens(contextWindow?.usedTokens)
									: "—"
							}
						/>
						{hasContextWindowData(contextWindow) ? (
							<InsightRow
								label="Estimated cost"
								value={formatCostUsd(contextWindow?.costUsd)}
							/>
						) : null}
					</dl>

					<h3 className="mt-2 text-sm font-medium">Recent sessions</h3>
					{recentSessions.length === 0 ? (
						<p className="text-sm text-muted-foreground">No sessions yet.</p>
					) : (
						<ul className="flex flex-col gap-2.5">
							{recentSessions.map((session, index) => (
								<li
									key={session.sessionId}
									className="flex items-center justify-between gap-3"
								>
									<span className="truncate text-sm">
										{sessionTitle(session, index)}
									</span>
									<span className="shrink-0 text-xs text-muted-foreground">
										{session.sessionId === activeSessionId ? "Active" : ""}
									</span>
								</li>
							))}
						</ul>
					)}
				</section>
			</div>

			{contextWindow?.breakdown.length ? (
				<section className="flex flex-col gap-3">
					<h3 className="text-sm font-medium">Context breakdown</h3>
					<ul className="grid grid-cols-1 gap-x-12 gap-y-3 sm:grid-cols-2">
						{contextWindow.breakdown.map((item) => (
							<li key={item.id} className="flex flex-col gap-1.5">
								<div className="flex items-center justify-between gap-3 text-sm">
									<span className="truncate">{item.label}</span>
									<span className="shrink-0 tabular-nums text-muted-foreground">
										{Math.round(item.percent)}%
									</span>
								</div>
								<div className="h-1 w-full overflow-hidden rounded-full bg-muted">
									<div
										className="h-full rounded-full bg-[var(--info)]"
										style={{
											width: `${Math.min(100, Math.max(2, item.percent))}%`,
										}}
									/>
								</div>
							</li>
						))}
					</ul>
				</section>
			) : null}

			<EditProfileDialog
				open={editOpen}
				onOpenChange={setEditOpen}
				initials={initials}
				name={name}
				handle={handle}
				avatarColor={avatarColor}
				avatarImage={avatarImage}
				onSave={({
					name: nextName,
					handle: nextHandle,
					avatarColor: nextColor,
					avatarImage: nextImage,
				}) => {
					setName(nextName)
					setHandle(nextHandle)
					setAvatarColor(nextColor)
					setAvatarImage(nextImage)
				}}
			/>
		</div>
	)
}