import { useAtom } from "jotai"
import { ShieldAlert } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
	labelPermissionOption,
	permissionKindLabel,
	presentPermissionRequest,
	sortPermissionOptions,
} from "@/lib/permission-presentation"
import { respondPermission } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import { activePermissionAtom, promptInFlightAtom, sessionStatusAtom } from "@/stores/atoms"

export function PermissionPrompt() {
	const [permission, setPermission] = useAtom(activePermissionAtom)
	const [, setSessionStatus] = useAtom(sessionStatusAtom)
	const [, setPromptInFlight] = useAtom(promptInFlightAtom)

	if (!permission) return null

	const presentation = presentPermissionRequest(permission)
	const options = sortPermissionOptions(permission.options)
	const kindLabel = permissionKindLabel(presentation.kind)

	async function handleDecision(optionId: string) {
		await respondPermission(permission!.requestId, optionId)
		setPermission(null)
		setPromptInFlight(true)
		setSessionStatus("generating")
	}

	return (
		<div className="px-3 py-3">
			<div className="mb-3 flex items-start gap-2.5">
				<ShieldAlert className="mt-0.5 size-4 shrink-0 text-[#3B5EF9]" />
				<div className="min-w-0 flex-1">
					<div className="mb-1 flex flex-wrap items-center gap-2">
						<p className="text-sm font-medium text-foreground">Permiso requerido</p>
						{kindLabel ? <Badge>{kindLabel}</Badge> : null}
					</div>
					<p className="text-xs text-muted-foreground">
						{presentation.toolLabel}
					</p>
				</div>
			</div>

			<div className="scrollbar-thin mb-3 max-h-32 overflow-y-auto rounded-md border border-popover-border bg-[#222222] px-3 py-2">
				<p className="whitespace-pre-wrap font-mono text-xs leading-relaxed text-foreground/90">
					{presentation.summary}
				</p>
				{presentation.detail ? (
					<p className="mt-1 truncate font-mono text-[11px] text-muted-foreground">
						{presentation.detail}
					</p>
				) : null}
			</div>

			<div className="flex flex-wrap gap-2">
				{options.map((option) => {
					const isReject = option.kind.includes("reject")
					const isAlways = option.kind.includes("always")
					return (
						<Button
							key={option.optionId}
							size="sm"
							variant={isReject ? "destructive" : isAlways ? "secondary" : "default"}
							className={cn(!isReject && !isAlways && "bg-[#3B5EF9] text-white hover:opacity-90")}
							onClick={() => void handleDecision(option.optionId)}
						>
							{labelPermissionOption(option)}
						</Button>
					)
				})}
			</div>
		</div>
	)
}