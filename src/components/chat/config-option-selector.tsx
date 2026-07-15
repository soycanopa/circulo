import { useAtom } from "jotai"
import { ChevronDown } from "lucide-react"
import { useMemo, useState } from "react"
import { InputGroupButton } from "@/components/ui/input-group"
import { setConfigOption } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import { configOptionsAtom } from "@/stores/atoms"
import type { ConfigOption } from "@/types/acp"

interface ConfigOptionSelectorProps {
	match: (option: ConfigOption) => boolean
	placeholder: string
	className?: string
	maxWidthClass?: string
	alwaysVisible?: boolean
	fallbackLabel?: string
}

export function ConfigOptionSelector({
	match,
	placeholder,
	className,
	maxWidthClass = "max-w-36",
	alwaysVisible = false,
	fallbackLabel,
}: ConfigOptionSelectorProps) {
	const [configOptions, setConfigOptions] = useAtom(configOptionsAtom)
	const [open, setOpen] = useState(false)

	const option = useMemo(() => configOptions.find(match), [configOptions, match])

	const currentName = useMemo(() => {
		if (!option) return fallbackLabel ?? null
		return (
			option.options.find((entry) => entry.value === option.currentValue)?.name ??
			option.currentValue
		)
	}, [option, fallbackLabel])

	const hasChoices = Boolean(option && option.options.length > 0)
	const isInteractive = hasChoices && (!alwaysVisible ? option!.options.length > 1 : true)

	if (!alwaysVisible) {
		if (!option || option.options.length === 0) return null
		if (option.options.length === 1) return null
	}

	async function handleSelect(value: string) {
		if (!option) return
		await setConfigOption(option.id, value)
		setConfigOptions((current) =>
			current.map((entry) =>
				entry.id === option.id ? { ...entry, currentValue: value } : entry,
			),
		)
		setOpen(false)
	}

	return (
		<div className={cn("relative", className)}>
			<InputGroupButton
				variant="ghost"
				size="sm"
				className={cn("h-7 gap-1 px-2 text-xs", maxWidthClass)}
				disabled={!isInteractive}
				onClick={() => isInteractive && setOpen((value) => !value)}
			>
				<span className="truncate">{currentName ?? placeholder}</span>
				<ChevronDown className="size-3 shrink-0 opacity-60" />
			</InputGroupButton>

			{open && option ? (
				<div className="absolute bottom-full left-0 z-30 mb-2 min-w-44 overflow-hidden rounded-lg border border-border bg-popover shadow-lg">
					<ul className="scrollbar-thin max-h-48 overflow-y-auto p-1">
						{option.options.map((entry) => (
							<li key={entry.value}>
								<button
									type="button"
									className={cn(
										"flex w-full flex-col rounded-md px-2 py-1.5 text-left text-xs hover:bg-accent",
										entry.value === option.currentValue && "bg-accent text-accent-foreground",
									)}
									onClick={() => void handleSelect(entry.value)}
								>
									<span>{entry.name}</span>
									{entry.description ? (
										<span className="text-[10px] text-muted-foreground">{entry.description}</span>
									) : null}
								</button>
							</li>
						))}
					</ul>
				</div>
			) : null}
		</div>
	)
}