import { useAtom } from "jotai"
import { ChevronDown } from "lucide-react"
import { useMemo, useRef, useState } from "react"
import { SelectorMenuItem } from "@/components/chat/selector-menu-item"
import { SelectorPortalMenu } from "@/components/chat/selector-portal-menu"
import { InputGroupButton } from "@/components/ui/input-group"
import {
	AGENT_MODE_PRESENTATIONS,
	resolveAgentModePresentation,
} from "@/lib/agent-mode-presentations"
import { findModeOption } from "@/lib/agent-mode"
import { setConfigOption } from "@/lib/tauri"
import { configOptionsAtom } from "@/stores/atoms"

export function AgentModeSelector() {
	const [configOptions, setConfigOptions] = useAtom(configOptionsAtom)
	const [open, setOpen] = useState(false)
	const rootRef = useRef<HTMLDivElement>(null)

	const modeOption = useMemo(() => findModeOption(configOptions), [configOptions])

	const entries = useMemo(() => {
		if (!modeOption) return []
		const order = new Map(AGENT_MODE_PRESENTATIONS.map((entry, index) => [entry.title, index]))
		return modeOption.options
			.map((entry) => ({
				...entry,
				presentation: resolveAgentModePresentation(entry.value, entry.name, entry.description),
			}))
			.sort(
				(a, b) =>
					(order.get(a.presentation.title) ?? 99) - (order.get(b.presentation.title) ?? 99),
			)
	}, [modeOption])

	const current = useMemo(() => {
		if (!modeOption) return resolveAgentModePresentation("plan", "Plan mode")
		const selected = entries.find((entry) => entry.value === modeOption.currentValue)
		return (
			selected?.presentation ??
			resolveAgentModePresentation(
				modeOption.currentValue,
				modeOption.options.find((entry) => entry.value === modeOption.currentValue)?.name,
			)
		)
	}, [entries, modeOption])

	const CurrentIcon = current.icon
	const hasChoices = entries.length > 0

	async function handleSelect(value: string) {
		if (!modeOption) return
		await setConfigOption(modeOption.id, value)
		setConfigOptions((current) =>
			current.map((entry) =>
				entry.id === modeOption.id ? { ...entry, currentValue: value } : entry,
			),
		)
		setOpen(false)
	}

	return (
		<div ref={rootRef} className="relative shrink-0">
			<InputGroupButton
				variant="ghost"
				size="sm"
				className="max-w-44 gap-1.5"
				disabled={!hasChoices}
				onClick={() => hasChoices && setOpen((value) => !value)}
			>
				<CurrentIcon className="size-3.5 shrink-0 opacity-80" />
				<span className="truncate">{current.title}</span>
				<ChevronDown className="size-3.5 shrink-0 opacity-60" />
			</InputGroupButton>

			<SelectorPortalMenu
				open={open && hasChoices}
				anchorRef={rootRef}
				onClose={() => setOpen(false)}
				minWidth={240}
				className="p-1"
			>
				<ul>
					{entries.map((entry) => {
						const Icon = entry.presentation.icon
						const isActive = entry.value === modeOption?.currentValue
						return (
							<li key={entry.value}>
								<SelectorMenuItem
									active={isActive}
									onClick={() => void handleSelect(entry.value)}
									className="items-start gap-2.5 py-2"
								>
									<Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
									<span className="min-w-0">
										<span className="block text-sm font-medium text-foreground">
											{entry.presentation.title}
										</span>
										{entry.presentation.description ? (
											<span className="mt-0.5 block text-xs leading-snug text-muted-foreground">
												{entry.presentation.description}
											</span>
										) : null}
									</span>
								</SelectorMenuItem>
							</li>
						)
					})}
				</ul>
			</SelectorPortalMenu>
		</div>
	)
}