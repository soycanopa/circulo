import { useAtom } from "jotai"
import { ChevronDown } from "lucide-react"
import { useMemo, useRef, useState } from "react"
import { SelectorMenuItem } from "@/components/chat/selector-menu-item"
import { SelectorPortalMenu } from "@/components/chat/selector-portal-menu"
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
	const rootRef = useRef<HTMLDivElement>(null)

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
		<div ref={rootRef} className={cn("relative shrink-0", className)}>
			<InputGroupButton
				variant="ghost"
				size="sm"
				className={cn("gap-1.5", maxWidthClass)}
				disabled={!isInteractive}
				onClick={() => isInteractive && setOpen((value) => !value)}
			>
				<span className="truncate">{currentName ?? placeholder}</span>
				<ChevronDown className="size-3.5 shrink-0 opacity-60" />
			</InputGroupButton>

			<SelectorPortalMenu
				open={open && Boolean(option)}
				anchorRef={rootRef}
				onClose={() => setOpen(false)}
				preferPlacement="above"
				minWidth={176}
				className="p-1"
			>
				<ul className="scrollbar-thin max-h-48 overflow-y-auto">
					{option?.options.map((entry) => (
						<li key={entry.value}>
							<SelectorMenuItem
								active={entry.value === option.currentValue}
								onClick={() => void handleSelect(entry.value)}
								className="flex-col items-start"
							>
								<span>{entry.name}</span>
								{entry.description ? (
									<span className="text-xs text-muted-foreground">{entry.description}</span>
								) : null}
							</SelectorMenuItem>
						</li>
					))}
				</ul>
			</SelectorPortalMenu>
		</div>
	)
}