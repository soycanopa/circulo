import { useAtom } from "jotai"
import { ExternalLink } from "lucide-react"
import {
	SettingsGroup,
	SettingsRow,
	SettingsSectionHeader,
	SettingsSelect,
} from "@/components/settings/settings-ui"
import { useAgentProviders } from "@/hooks/use-agent-providers"
import { useAgentSwitch } from "@/hooks/use-agent-switch"
import { persistAppSettings, type DefaultAgentProvider } from "@/lib/app-settings"
import type { AgentProviderId } from "@/lib/agent-providers"
import { appSettingsAtom, projectPathAtom } from "@/stores/atoms"

export function AgentsSettings() {
	const [settings, setSettings] = useAtom(appSettingsAtom)
	const [projectPath] = useAtom(projectPathAtom)
	const { entries, loading } = useAgentProviders()
	const { switchAgent } = useAgentSwitch()

	const selectable = entries.filter((entry) => entry.selectable)

	async function handleDefaultChange(value: string) {
		const agentId = value as AgentProviderId
		setSettings(persistAppSettings({ defaultProvider: agentId as DefaultAgentProvider }))
		if (projectPath) {
			await switchAgent(agentId, projectPath)
		}
	}

	return (
		<div className="space-y-6">
			<div>
				<SettingsSectionHeader
					title="Agente activo"
					description="App CLI que Circulo usa para hablar con el modelo vía ACP. También disponible en el app bar."
				/>
				<SettingsGroup>
					<SettingsRow
						label="App por defecto"
						description="Se usa al abrir proyectos y en sesiones nuevas."
					>
						<SettingsSelect
							value={settings.defaultProvider}
							disabled={selectable.length === 0}
							onChange={(value) => void handleDefaultChange(value)}
							options={
								selectable.length > 0
									? selectable.map((entry) => ({
											value: entry.id,
											label: entry.label,
										}))
									: [{ value: "opencode", label: "OpenCode (instalar CLI)", disabled: true }]
							}
						/>
					</SettingsRow>
				</SettingsGroup>
			</div>

			<div>
				<SettingsSectionHeader
					title="Apps detectadas"
					description="Circulo comprueba qué CLIs tienes instaladas en el sistema."
				/>
				<SettingsGroup>
					{loading ? (
						<SettingsRow label="Estado">
							<span className="text-xs text-muted-foreground">Comprobando…</span>
						</SettingsRow>
					) : (
						entries.map((entry) => (
							<SettingsRow
								key={entry.id}
								label={entry.label}
								description={entry.command}
							>
								<span className="text-right text-xs text-muted-foreground">
									{!entry.installed ? (
										<span className="text-destructive/90">No instalada</span>
									) : !entry.acpReady ? (
										<span>Instalada · ACP próximamente</span>
									) : (
										<span className="font-mono">{entry.version ?? "OK"}</span>
									)}
								</span>
							</SettingsRow>
						))
					)}
				</SettingsGroup>
			</div>

			<div>
				<SettingsSectionHeader
					title="Instalación"
					description="Cada app usa su propio CLI. Circulo solo orquesta vía ACP stdio."
				/>
				<SettingsGroup>
					<SettingsRow label="OpenCode">
						<a
							href="https://opencode.ai/docs/acp/"
							target="_blank"
							rel="noreferrer"
							className="inline-flex items-center gap-1 text-xs text-foreground underline-offset-2 hover:underline"
						>
							Documentación ACP
							<ExternalLink className="size-3" />
						</a>
					</SettingsRow>
					<SettingsRow label="Grok Build">
						<a
							href="https://x.ai"
							target="_blank"
							rel="noreferrer"
							className="inline-flex items-center gap-1 text-xs text-foreground underline-offset-2 hover:underline"
						>
							x.ai
							<ExternalLink className="size-3" />
						</a>
					</SettingsRow>
				</SettingsGroup>
			</div>
		</div>
	)
}