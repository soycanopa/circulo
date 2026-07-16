import { getDefaultAgentMode } from "@/lib/app-settings"
import { getLastModel } from "@/lib/preferences"
import { setConfigOption } from "@/lib/tauri"
import type { ConfigOption } from "@/types/acp"

function findModeOption(options: ConfigOption[]) {
	return options.find(
		(option) => option.id === "mode" || option.category?.toLowerCase() === "mode",
	)
}

function findModelOption(options: ConfigOption[]) {
	return options.find(
		(option) =>
			option.id === "model" || option.category?.toLowerCase().includes("model"),
	)
}

/** Apply Circulo defaults after a session becomes ready. */
export async function applySessionDefaults(configOptions: ConfigOption[]): Promise<void> {
	const modeOption = findModeOption(configOptions)
	const preferredMode = getDefaultAgentMode()
	if (
		modeOption?.options.some((entry) => entry.value === preferredMode) &&
		modeOption.currentValue !== preferredMode
	) {
		await setConfigOption(modeOption.id, preferredMode)
	}

	const modelOption = findModelOption(configOptions)
	const lastModel = getLastModel()
	if (
		modelOption &&
		lastModel &&
		modelOption.options.some((entry) => entry.value === lastModel) &&
		modelOption.currentValue !== lastModel
	) {
		await setConfigOption(modelOption.id, lastModel)
	}
}