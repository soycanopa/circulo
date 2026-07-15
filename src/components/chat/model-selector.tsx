import { useAtom } from "jotai"
import { setConfigOption } from "@/lib/tauri"
import { configOptionsAtom } from "@/stores/atoms"

export function ModelSelector() {
	const [configOptions, setConfigOptions] = useAtom(configOptionsAtom)
	const modelOption = configOptions.find(
		(option) => option.category?.toLowerCase().includes("model") || option.id === "model",
	)

	if (!modelOption || modelOption.options.length === 0) return null

	async function handleChange(value: string) {
		await setConfigOption(modelOption!.id, value)
		setConfigOptions((current) =>
			current.map((option) =>
				option.id === modelOption!.id
					? { ...option, currentValue: value }
					: option,
			),
		)
	}

	return (
		<label className="flex items-center gap-2 text-xs text-muted-foreground">
			<span>Modelo</span>
			<select
				className="rounded-md border border-border bg-muted px-2 py-1 text-sm text-foreground"
				value={modelOption.currentValue}
				onChange={(event) => handleChange(event.target.value)}
			>
				{modelOption.options.map((option) => (
					<option key={option.value} value={option.value}>
						{option.name}
					</option>
				))}
			</select>
		</label>
	)
}