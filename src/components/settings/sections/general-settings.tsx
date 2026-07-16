import { open } from "@tauri-apps/plugin-dialog"
import { useAtom, useAtomValue } from "jotai"
import { useMemo, useState } from "react"
import {
	SettingsGroup,
	SettingsRow,
	SettingsSectionHeader,
	SettingsSelect,
	SettingsToggle,
} from "@/components/settings/settings-ui"
import { AGENT_MODE_PRESENTATIONS } from "@/lib/agent-mode-presentations"
import { persistAppSettings } from "@/lib/app-settings"
import { buildModelGroups, findModelEntry } from "@/lib/model-groups"
import { getLastModel, setLastModel } from "@/lib/preferences"
import { getProjectDisplayName } from "@/lib/project-display"
import { appSettingsAtom, configOptionsAtom } from "@/stores/atoms"

export function GeneralSettings() {
	const configOptions = useAtomValue(configOptionsAtom)
	const [settings, setSettings] = useAtom(appSettingsAtom)
	const [lastModel, setLastModelState] = useState(getLastModel)

	const modeOption = configOptions.find(
		(option) => option.id === "mode" || option.category?.toLowerCase() === "mode",
	)
	const modelOption = configOptions.find(
		(option) => option.category?.toLowerCase().includes("model") || option.id === "model",
	)

	const modeChoices = useMemo(() => {
		if (modeOption?.options.length) {
			return modeOption.options.map((entry) => ({
				value: entry.value,
				label: entry.name?.trim() || entry.value,
			}))
		}
		return AGENT_MODE_PRESENTATIONS.map((entry) => ({
			value: entry.values[0]!,
			label: entry.title,
		}))
	}, [modeOption])

	const modelGroups = useMemo(
		() => (modelOption ? buildModelGroups(modelOption.options) : []),
		[modelOption],
	)

	const modelChoices = useMemo(
		() =>
			modelGroups.flatMap((group) =>
				group.models.map((model) => ({
					value: model.value,
					label: model.name,
				})),
			),
		[modelGroups],
	)

	const selectedModelLabel = useMemo(() => {
		if (!lastModel) return "Sin preferencia"
		const entry = findModelEntry(modelGroups, lastModel)
		return entry?.name ?? lastModel
	}, [lastModel, modelGroups])

	function patchSettings(patch: Parameters<typeof persistAppSettings>[0]) {
		setSettings(persistAppSettings(patch))
	}

	async function handlePickChatsFolder() {
		const selected = await open({
			directory: true,
			multiple: false,
			title: "Carpeta de Chats",
		})
		if (!selected || Array.isArray(selected)) return
		patchSettings({ chatsProjectPath: selected })
	}

	return (
		<div className="space-y-6">
			<div>
				<SettingsSectionHeader
					title="Sesiones nuevas"
					description="Valores por defecto al crear o cargar una sesión."
				/>
				<SettingsGroup>
					<SettingsRow
						label="Modo agente por defecto"
						description="Se aplica cuando el agente expone ese modo."
					>
						<SettingsSelect
							value={settings.defaultAgentMode}
							onChange={(value) => patchSettings({ defaultAgentMode: value })}
							options={modeChoices}
						/>
					</SettingsRow>
					<SettingsRow
						label="Modelo por defecto"
						description={modelChoices.length ? selectedModelLabel : "Conecta un agente para ver modelos."}
					>
						<SettingsSelect
							value={lastModel ?? (modelChoices[0]?.value ?? "")}
							disabled={modelChoices.length === 0}
							onChange={(value) => {
								setLastModel(value)
								setLastModelState(value)
							}}
							options={
								modelChoices.length > 0
									? modelChoices
									: [{ value: "", label: "Sin modelos", disabled: true }]
							}
						/>
					</SettingsRow>

				</SettingsGroup>
			</div>

			<div>
				<SettingsSectionHeader
					title="Proyectos y sidebar"
					description="Carpeta de chats y secciones visibles en el panel lateral."
				/>
				<SettingsGroup>
					<SettingsRow
						label="Carpeta de Chats"
						description={getProjectDisplayName(settings.chatsProjectPath)}
					>
						<button
							type="button"
							onClick={() => void handlePickChatsFolder()}
							className="h-8 rounded-md border border-border bg-background px-3 text-xs text-foreground transition-colors hover:bg-accent"
						>
							Elegir carpeta
						</button>
					</SettingsRow>
					<SettingsRow
						label="Mostrar Chats en sidebar"
						description="Carpeta de sesiones generales sin proyecto."
					>
						<SettingsToggle
							checked={settings.showChatsInSidebar}
							ariaLabel="Mostrar Chats en sidebar"
							onChange={(checked) => patchSettings({ showChatsInSidebar: checked })}
						/>
					</SettingsRow>
					<SettingsRow
						label="Mostrar Pinned en sidebar"
						description="Oculta la sección si no usas sesiones fijadas."
					>
						<SettingsToggle
							checked={settings.showPinnedInSidebar}
							ariaLabel="Mostrar Pinned en sidebar"
							onChange={(checked) => patchSettings({ showPinnedInSidebar: checked })}
						/>
					</SettingsRow>
				</SettingsGroup>
			</div>
		</div>
	)
}