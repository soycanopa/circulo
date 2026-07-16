import { useAtomValue } from "jotai"
import { getSettingsSectionMeta } from "@/lib/app-settings"
import { settingsSectionAtom } from "@/stores/atoms"

export function SettingsTitle() {
	const section = useAtomValue(settingsSectionAtom)
	const meta = getSettingsSectionMeta(section)

	return (
		<span className="flex min-w-0 flex-1 items-center text-xs leading-none">
			<span className="text-muted-foreground/80">Settings</span>
			<span className="mx-1.5 shrink-0 text-muted-foreground/40">/</span>
			<span className="truncate text-foreground/90">{meta.label}</span>
		</span>
	)
}