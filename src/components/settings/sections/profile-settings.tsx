import { useAtomValue } from "jotai"
import { useMemo } from "react"
import {
	SettingsEmptyState,
	SettingsGroup,
	SettingsRow,
	SettingsSectionHeader,
} from "@/components/settings/settings-ui"
import { getArchivedSessionIds } from "@/lib/archived-sessions"
import {
	formatContextTokens,
	formatCostUsd,
	hasContextWindowData,
} from "@/lib/context-window"
import { getFavoriteModels } from "@/lib/favorite-models"
import { getLastModel } from "@/lib/preferences"
import { getProjectDisplayName } from "@/lib/project-display"
import { getRecentProjects } from "@/lib/recent-projects"
import { getPinnedSessionIds } from "@/lib/pinned-sessions"
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

function formatSessionTime(session: SessionInfo): string {
	if (!session.updatedAt) return "—"
	const parsed = Date.parse(session.updatedAt)
	return Number.isNaN(parsed) ? "—" : new Date(parsed).toLocaleString()
}

function StatCard({ label, value }: { label: string; value: string }) {
	return (
		<div className="rounded-lg border border-border/60 bg-muted/20 px-3 py-2.5">
			<p className="text-[10px] uppercase tracking-wide text-muted-foreground">{label}</p>
			<p className="mt-1 text-sm font-medium text-foreground">{value}</p>
		</div>
	)
}

export function ProfileSettings() {
	const sessions = useAtomValue(sessionsAtom)
	const activeSessionId = useAtomValue(activeSessionIdAtom)
	const projectPath = useAtomValue(projectPathAtom)
	const contextWindow = useAtomValue(contextWindowAtom)

	const activeIndex = sessions.findIndex((session) => session.sessionId === activeSessionId)
	const activeSession = activeIndex >= 0 ? sessions[activeIndex] : null

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

	return (
		<div className="space-y-6">
			<div>
				<SettingsSectionHeader
					title="Actividad"
					description="Resumen local de tu uso en Circulo."
				/>
				<div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
					<StatCard label="Sesiones" value={String(sessions.length)} />
					<StatCard label="Proyectos" value={String(projectCount)} />
					<StatCard label="Favoritos" value={String(favoriteCount)} />
					<StatCard label="Pinned" value={String(pinnedCount)} />
					<StatCard label="Archivadas" value={String(archivedCount)} />
					<StatCard
						label="Contexto"
						value={
							hasContextWindowData(contextWindow)
								? formatContextTokens(contextWindow?.usedTokens)
								: "—"
						}
					/>
				</div>
			</div>

			<div>
				<SettingsSectionHeader title="Sesión activa" />
				<SettingsGroup>
					<SettingsRow label="Proyecto">
						<span className="max-w-[200px] truncate text-xs text-muted-foreground">
							{getProjectDisplayName(projectPath)}
						</span>
					</SettingsRow>
					<SettingsRow label="Sesión">
						<span className="max-w-[200px] truncate text-xs text-muted-foreground">
							{activeSession
								? sessionTitle(activeSession, activeIndex)
								: "Sin sesión activa"}
						</span>
					</SettingsRow>
					<SettingsRow label="Último modelo">
						<span className="text-xs text-muted-foreground">
							{getLastModel() ?? "—"}
						</span>
					</SettingsRow>
					{hasContextWindowData(contextWindow) ? (
						<SettingsRow label="Coste estimado">
							<span className="text-xs text-muted-foreground">
								{formatCostUsd(contextWindow?.costUsd)}
							</span>
						</SettingsRow>
					) : null}
				</SettingsGroup>
			</div>

			<div>
				<SettingsSectionHeader title="Sesiones recientes" />
				{recentSessions.length === 0 ? (
					<SettingsEmptyState>No hay sesiones todavía.</SettingsEmptyState>
				) : (
					<SettingsGroup>
						{recentSessions.map((session, index) => (
							<SettingsRow
								key={session.sessionId}
								label={sessionTitle(session, index)}
								description={formatSessionTime(session)}
							>
								<span className="text-[10px] text-muted-foreground">
									{session.sessionId === activeSessionId ? "Activa" : ""}
								</span>
							</SettingsRow>
						))}
					</SettingsGroup>
				)}
			</div>

			{contextWindow?.breakdown.length ? (
				<div>
					<SettingsSectionHeader title="Context breakdown" />
					<SettingsGroup>
						{contextWindow.breakdown.map((item) => (
							<SettingsRow key={item.id} label={item.label}>
								<span className="text-xs text-muted-foreground">
									{Math.round(item.percent)}%
								</span>
							</SettingsRow>
						))}
					</SettingsGroup>
				</div>
			) : null}
		</div>
	)
}