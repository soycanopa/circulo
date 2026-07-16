import { useAtom, useAtomValue } from "jotai"
import { ChevronDown, Loader2 } from "lucide-react"
import { useMemo, useRef, useState } from "react"
import { AgentAppIcon } from "@/components/chat/agent-app-icon"
import { SelectorMenuItem } from "@/components/chat/selector-menu-item"
import { SelectorPortalMenu } from "@/components/chat/selector-portal-menu"
import { useAgentProviders } from "@/hooks/use-agent-providers"
import { useAgentSwitch } from "@/hooks/use-agent-switch"
import { chromeIconButtonClass } from "@/lib/control-button"
import type { AgentProviderId } from "@/lib/agent-providers"
import { cn } from "@/lib/utils"
import { activeAgentIdAtom, projectPathAtom, promptInFlightAtom } from "@/stores/atoms"

export function AgentAppSelector() {
	const [activeAgentId, setActiveAgentId] = useAtom(activeAgentIdAtom)
	const projectPath = useAtomValue(projectPathAtom)
	const promptInFlight = useAtomValue(promptInFlightAtom)
	const { entries, loading, getEntry } = useAgentProviders()
	const { switchAgent } = useAgentSwitch()
	const [open, setOpen] = useState(false)
	const [switching, setSwitching] = useState(false)
	const rootRef = useRef<HTMLDivElement>(null)

	const current = getEntry(activeAgentId) ?? entries[0]
	const installedEntries = useMemo(
		() => entries.filter((entry) => entry.installed),
		[entries],
	)

	async function handleSelect(agentId: AgentProviderId) {
		const entry = getEntry(agentId)
		if (!entry?.selectable || agentId === activeAgentId) {
			setOpen(false)
			return
		}

		setSwitching(true)
		try {
			const ok = await switchAgent(agentId, projectPath)
			if (ok) setActiveAgentId(agentId)
		} finally {
			setSwitching(false)
			setOpen(false)
		}
	}

	if (!current) return null

	return (
		<div ref={rootRef} className="relative shrink-0">
			<button
				type="button"
				disabled={switching || promptInFlight}
				onClick={() => setOpen((value) => !value)}
				title={`Agente: ${current.label}`}
				aria-haspopup="listbox"
				aria-expanded={open}
				className={cn(
					chromeIconButtonClass,
					"h-7 max-w-[9.5rem] gap-1.5 pl-1.5 pr-2 text-[11px] font-medium text-foreground/90",
					open && "bg-accent text-foreground",
				)}
			>
				<span className="flex size-5 shrink-0 items-center justify-center rounded-md bg-muted/50">
					{switching || loading ? (
						<Loader2 className="size-3 animate-spin text-muted-foreground" />
					) : (
						<AgentAppIcon agentId={current.id} />
					)}
				</span>
				<span className="min-w-0 truncate">{current.shortLabel}</span>
				<ChevronDown
					className={cn(
						"size-3 shrink-0 text-muted-foreground transition-transform",
						open && "rotate-180",
					)}
				/>
			</button>

			<SelectorPortalMenu
				open={open}
				anchorRef={rootRef}
				onClose={() => setOpen(false)}
				minWidth={220}
				preferPlacement="below"
			>
				<div className="p-1">
					<p className="px-2 py-1.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
						Apps instaladas
					</p>
					{entries.map((entry) => {
						const disabled = !entry.selectable
						const statusLabel = !entry.installed
							? "No instalada"
							: !entry.acpReady
								? "Próximamente"
								: entry.version ?? "Instalada"

						return (
							<SelectorMenuItem
								key={entry.id}
								active={entry.id === activeAgentId}
								onClick={() => {
									if (!disabled) void handleSelect(entry.id)
								}}
								className={cn(
									"items-start gap-2 py-2",
									disabled && "cursor-not-allowed opacity-50 hover:bg-transparent",
								)}
							>
								<span className="mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-md bg-muted/50">
									<AgentAppIcon agentId={entry.id} />
								</span>
								<span className="min-w-0 flex-1">
									<span className="block truncate text-xs font-medium">{entry.label}</span>
									<span className="mt-0.5 block truncate text-[10px] text-muted-foreground">
										{statusLabel}
									</span>
								</span>
							</SelectorMenuItem>
						)
					})}
					{installedEntries.length === 0 ? (
						<p className="px-2 py-2 text-[11px] text-muted-foreground">
							Instala OpenCode para empezar.
						</p>
					) : null}
				</div>
			</SelectorPortalMenu>
		</div>
	)
}