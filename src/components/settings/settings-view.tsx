import { useAtomValue } from "jotai"
import { SettingsPanel } from "@/components/settings/settings-panel"
import { getSettingsSectionMeta } from "@/lib/app-settings"
import { settingsSectionAtom } from "@/stores/atoms"

export function SettingsView() {
	const section = useAtomValue(settingsSectionAtom)
	const meta = getSettingsSectionMeta(section)

	return (
		<div className="flex h-full min-h-0 flex-col">
			<div className="scrollbar-thin min-h-0 flex-1 overflow-y-auto px-6 py-5 md:px-10 md:py-7">
				<div className="mx-auto w-full max-w-5xl">
					<header className="mb-6 border-b border-border/50 pb-5">
						<p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
							Settings
						</p>
						<h1 className="mt-1 text-lg font-medium text-foreground">{meta.label}</h1>
						<p className="mt-1 text-sm text-muted-foreground">{meta.description}</p>
					</header>
					<SettingsPanel section={section} />
				</div>
			</div>
		</div>
	)
}