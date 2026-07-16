import { SettingsGroup, SettingsRow, SettingsSectionHeader } from "@/components/settings/settings-ui"

const SHORTCUTS = [
	{ keys: "⌘ B", description: "Mostrar u ocultar sidebar", windowsKeys: "Ctrl B" },
	{ keys: "Esc", description: "Cerrar overlays y settings" },
	{ keys: "Enter", description: "Enviar mensaje en el composer" },
	{ keys: "Shift Enter", description: "Nueva línea en el composer" },
] as const

export function ShortcutsSettings() {
	const isMac =
		typeof navigator !== "undefined" &&
		(/Mac|iPhone|iPad|iPod/.test(navigator.platform) ||
			navigator.userAgent.includes("Mac OS X"))

	return (
		<div className="space-y-6">
			<div>
				<SettingsSectionHeader
					title="Atajos de teclado"
					description="Personalización vía JSON llegará en una versión futura."
				/>
				<SettingsGroup>
					{SHORTCUTS.map((shortcut) => (
						<SettingsRow
							key={shortcut.description}
							label={shortcut.description}
						>
							<kbd className="rounded border border-border bg-muted px-2 py-1 font-mono text-[10px] text-foreground">
								{"windowsKeys" in shortcut && !isMac
									? shortcut.windowsKeys
									: shortcut.keys}
							</kbd>
						</SettingsRow>
					))}
				</SettingsGroup>
			</div>

			<p className="text-xs text-muted-foreground">
				Próximamente: archivo <code className="font-mono">~/.circulo/keybindings.json</code>{" "}
				para reasignar comandos.
			</p>
		</div>
	)
}