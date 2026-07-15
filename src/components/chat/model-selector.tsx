import { useAtom } from "jotai"
import { ChevronDown } from "lucide-react"
import { useMemo, useRef, useState } from "react"
import { useDismissOnOutside } from "@/hooks/use-dismiss-on-outside"
import { InputGroupButton } from "@/components/ui/input-group"
import { setLastModel } from "@/lib/preferences"
import { setConfigOption } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import { configOptionsAtom } from "@/stores/atoms"

export function ModelSelector() {
	const [configOptions, setConfigOptions] = useAtom(configOptionsAtom)
	const [open, setOpen] = useState(false)
	const [query, setQuery] = useState("")
	const rootRef = useRef<HTMLDivElement>(null)

	useDismissOnOutside(rootRef, () => setOpen(false), open)

	const modelOption = configOptions.find(
		(option) => option.category?.toLowerCase().includes("model") || option.id === "model",
	)

	const currentName = useMemo(() => {
		if (!modelOption) return "Modelo"
		return (
			modelOption.options.find((o) => o.value === modelOption.currentValue)?.name ??
			modelOption.currentValue
		)
	}, [modelOption])

	const filtered = useMemo(() => {
		if (!modelOption) return []
		const q = query.trim().toLowerCase()
		if (!q) return modelOption.options
		return modelOption.options.filter(
			(o) => o.name.toLowerCase().includes(q) || o.value.toLowerCase().includes(q),
		)
	}, [modelOption, query])

	const hasModels = Boolean(modelOption && modelOption.options.length > 0)

	async function handleSelect(value: string) {
		if (!modelOption) return
		await setConfigOption(modelOption.id, value)
		setLastModel(value)
		setConfigOptions((current) =>
			current.map((option) =>
				option.id === modelOption.id ? { ...option, currentValue: value } : option,
			),
		)
		setOpen(false)
		setQuery("")
	}

	return (
		<div ref={rootRef} className="relative">
			<InputGroupButton
				variant="ghost"
				size="sm"
				className="h-7 max-w-48 gap-1 px-2 text-xs"
				disabled={!hasModels}
				onClick={() => hasModels && setOpen((v) => !v)}
			>
				<span className="truncate">{currentName}</span>
				<ChevronDown className="size-3 shrink-0 opacity-60" />
			</InputGroupButton>

			{open && modelOption ? (
				<div className="absolute bottom-full left-0 z-30 mb-2 w-64 overflow-hidden rounded-lg border border-border bg-popover shadow-lg">
					<div className="border-b border-border p-2">
						<input
							value={query}
							onChange={(e) => setQuery(e.target.value)}
							placeholder="Buscar modelo…"
							className="h-7 w-full rounded-md border border-input bg-muted/40 px-2 text-xs outline-none focus:ring-1 focus:ring-ring/40"
							autoFocus
						/>
					</div>
					<ul className="scrollbar-thin max-h-48 overflow-y-auto p-1">
						{filtered.length === 0 ? (
							<li className="px-2 py-2 text-xs text-muted-foreground">Sin resultados</li>
						) : (
							filtered.map((option) => (
								<li key={option.value}>
									<button
										type="button"
										className={cn(
											"flex w-full rounded-md px-2 py-1.5 text-left text-xs hover:bg-accent",
											option.value === modelOption.currentValue &&
												"bg-accent text-accent-foreground",
										)}
										onClick={() => void handleSelect(option.value)}
									>
										{option.name}
									</button>
								</li>
							))
						)}
					</ul>
				</div>
			) : null}
		</div>
	)
}