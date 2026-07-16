import { useAtomValue } from "jotai"
import { useCallback, useEffect, useState } from "react"
import {
	SettingsEmptyState,
	SettingsGroup,
	SettingsRow,
	SettingsSectionHeader,
} from "@/components/settings/settings-ui"
import { getProjectDisplayName } from "@/lib/project-display"
import { listOpencodeSkills, type OpencodeSkillEntry } from "@/lib/tauri"
import { projectPathAtom } from "@/stores/atoms"

function SkillsList({ skills, emptyLabel }: { skills: OpencodeSkillEntry[]; emptyLabel: string }) {
	if (skills.length === 0) {
		return <SettingsEmptyState>{emptyLabel}</SettingsEmptyState>
	}

	return (
		<SettingsGroup>
			{skills.map((skill) => (
				<SettingsRow
					key={`${skill.scope}-${skill.path}`}
					label={skill.name}
					description={skill.description ?? skill.path}
				>
					<span className="rounded border border-border px-1.5 py-0.5 text-[10px] uppercase text-muted-foreground">
						{skill.scope}
					</span>
				</SettingsRow>
			))}
		</SettingsGroup>
	)
}

export function SkillsSettings() {
	const projectPath = useAtomValue(projectPathAtom)
	const [skills, setSkills] = useState<OpencodeSkillEntry[]>([])
	const [loading, setLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)

	const refresh = useCallback(async () => {
		setLoading(true)
		setError(null)
		try {
			const entries = await listOpencodeSkills(projectPath)
			setSkills(entries)
		} catch (err) {
			setError(err instanceof Error ? err.message : "No se pudieron cargar los skills")
		} finally {
			setLoading(false)
		}
	}, [projectPath])

	useEffect(() => {
		void refresh()
	}, [refresh])

	const globalSkills = skills.filter((skill) => skill.scope === "global")
	const projectSkills = skills.filter((skill) => skill.scope === "project")

	return (
		<div className="space-y-6">
			<div>
				<SettingsSectionHeader
					title="Skills de OpenCode"
					description="Lectura desde ~/.config/opencode/skills y .opencode/skills del proyecto."
				/>
				{loading ? (
					<SettingsEmptyState>Cargando skills…</SettingsEmptyState>
				) : error ? (
					<SettingsEmptyState>{error}</SettingsEmptyState>
				) : (
					<div className="space-y-5">
						<div>
							<p className="mb-2 text-xs font-medium text-muted-foreground">Global</p>
							<SkillsList
								skills={globalSkills}
								emptyLabel="No hay skills globales instalados."
							/>
						</div>
						<div>
							<p className="mb-2 text-xs font-medium text-muted-foreground">
								Proyecto — {getProjectDisplayName(projectPath)}
							</p>
							<SkillsList
								skills={projectSkills}
								emptyLabel="No hay skills en .opencode/skills de este proyecto."
							/>
						</div>
					</div>
				)}
			</div>

			<p className="text-xs text-muted-foreground">
				Instalación global de skills — próximamente. Por ahora agrega carpetas con{" "}
				<code className="font-mono">SKILL.md</code> en las rutas de OpenCode.
			</p>
		</div>
	)
}