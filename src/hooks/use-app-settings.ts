import { useSetAtom } from "jotai"
import { useEffect } from "react"
import { getAppSettings } from "@/lib/tauri"
import { appSettingsAtom } from "@/stores/atoms"

export function useAppSettings() {
	const setSettings = useSetAtom(appSettingsAtom)

	useEffect(() => {
		void getAppSettings()
			.then(setSettings)
			.catch(() => setSettings(null))
	}, [setSettings])
}
