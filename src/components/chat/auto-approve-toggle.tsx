import { useAtomValue, useSetAtom } from "jotai"
import { Shield, ShieldCheck } from "lucide-react"
import { useCallback } from "react"
import {
	Popover,
	PopoverAnchor,
	PopoverContent,
} from "@/components/ui/popover"
import { useHoverPopover } from "@/hooks/use-hover-popover"
import {
	autoApproveConfigValue,
	findAutoApproveConfigOption,
	isAutoApproveConfigEnabled,
} from "@/lib/auto-approve"
import { setAutoApprove, setConfigOption } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import {
	activeSessionIdAtom,
	appSettingsAtom,
	sessionsAtom,
	visibleConfigOptionsAtom,
	visiblePromptInFlightAtom,
} from "@/stores/atoms"

interface AutoApproveToggleProps {
	className?: string
}

export function AutoApproveToggle({ className }: AutoApproveToggleProps) {
	const options = useAtomValue(visibleConfigOptionsAtom)
	const promptInFlight = useAtomValue(visiblePromptInFlightAtom)
	const appSettings = useAtomValue(appSettingsAtom)
	const setAppSettings = useSetAtom(appSettingsAtom)
	const setSessions = useSetAtom(sessionsAtom)
	const activeSessionId = useAtomValue(activeSessionIdAtom)
	const { open, setOpen, showPopover, scheduleClose } = useHoverPopover()

	const configOption = findAutoApproveConfigOption(options)
	const enabled = configOption
		? isAutoApproveConfigEnabled(configOption)
		: (appSettings?.autoApproveEnabled ?? false)

	const handleToggle = useCallback(async () => {
		const next = !enabled

		if (configOption) {
			const value = autoApproveConfigValue(configOption, next)
			if (!value) return

			const targetSid = activeSessionId
			if (targetSid) {
				setSessions((prev) => {
					const current = prev[targetSid]
					if (!current) return prev
					return {
						...prev,
						[targetSid]: {
							...current,
							configOptions: current.configOptions.map((entry) =>
								entry.id === configOption.id
									? { ...entry, currentValue: value }
									: entry,
							),
						},
					}
				})
			}
			void setConfigOption(configOption.id, value)
		}

		try {
			const settings = await setAutoApprove(next)
			setAppSettings(settings)
		} catch {
			// Config may still have applied even if settings persistence failed.
		}
	}, [
		activeSessionId,
		configOption,
		enabled,
		setAppSettings,
		setSessions,
	])

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverAnchor asChild>
				<button
					type="button"
					disabled={promptInFlight}
					aria-label={
						enabled
							? "Edición automática activada — el agente no pedirá permiso"
							: "Edición automática desactivada — el agente pedirá permiso"
					}
					onClick={() => void handleToggle()}
					onMouseEnter={showPopover}
					onMouseLeave={scheduleClose}
					onFocus={showPopover}
					onBlur={scheduleClose}
					className={cn(
						"inline-flex shrink-0 items-center justify-center rounded-md p-1 transition-colors",
						"focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-white/15",
						"disabled:cursor-not-allowed disabled:opacity-50",
						enabled
							? "text-emerald-300 hover:bg-emerald-500/10"
							: "text-white/40 hover:bg-white/[0.08] hover:text-white/70",
						className,
					)}
				>
					{enabled ? (
						<ShieldCheck className="size-4" aria-hidden />
					) : (
						<Shield className="size-4" aria-hidden />
					)}
				</button>
			</PopoverAnchor>
			<PopoverContent
				align="start"
				sideOffset={6}
				className="w-56 p-3"
				onMouseEnter={showPopover}
				onMouseLeave={scheduleClose}
				onOpenAutoFocus={(event) => event.preventDefault()}
			>
				<p className="text-xs font-medium text-fg">Edición automática</p>
				<p className="mt-1.5 text-xs leading-snug text-white/60">
					{enabled
						? "El agente puede editar sin pedir permiso en cada cambio."
						: "El agente pedirá permiso antes de editar archivos."}
				</p>
				<p
					className={cn(
						"mt-2 text-[10px]",
						enabled ? "text-emerald-300/80" : "text-white/40",
					)}
				>
					Clic para {enabled ? "desactivar" : "activar"}
				</p>
			</PopoverContent>
		</Popover>
	)
}
