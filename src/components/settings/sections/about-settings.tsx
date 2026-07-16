import { useAtomValue } from "jotai"
import { SettingsGroup, SettingsRow, SettingsSectionHeader } from "@/components/settings/settings-ui"
import { projectPathAtom } from "@/stores/atoms"

import { APP_VERSION } from "@/lib/version"
const REPO_URL = "https://github.com/soycanopa/circulo"

export function AboutSettings() {
	const projectPath = useAtomValue(projectPathAtom)

	return (
		<div className="space-y-6">
			<div>
				<SettingsSectionHeader
					title="Circulo"
					description="Desktop AI orchestrator via Agent Client Protocol."
				/>
				<SettingsGroup>
					<SettingsRow label="Versión">
						<span className="text-xs text-muted-foreground">{APP_VERSION}</span>
					</SettingsRow>
					<SettingsRow label="Licencia">
						<span className="text-xs text-muted-foreground">MIT</span>
					</SettingsRow>
					<SettingsRow label="Autor">
						<span className="text-xs text-muted-foreground">Carlos Andres O. P.</span>
					</SettingsRow>
					<SettingsRow label="Repositorio">
						<a
							href={REPO_URL}
							target="_blank"
							rel="noreferrer"
							className="text-xs text-foreground underline-offset-2 hover:underline"
						>
							github.com/soycanopa/circulo
						</a>
					</SettingsRow>
				</SettingsGroup>
			</div>

			<div>
				<SettingsSectionHeader title="Sesión" />
				<SettingsGroup>
					<SettingsRow label="Proyecto abierto">
						<span className="max-w-[220px] truncate font-mono text-xs text-muted-foreground">
							{projectPath ?? "—"}
						</span>
					</SettingsRow>
				</SettingsGroup>
			</div>
		</div>
	)
}