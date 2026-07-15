import { useAtom } from "jotai"
import { ShieldAlert } from "lucide-react"
import { Button } from "@/components/ui/button"
import { respondPermission } from "@/lib/tauri"
import { activePermissionAtom, sessionStatusAtom } from "@/stores/atoms"

export function PermissionCard() {
	const [permission, setPermission] = useAtom(activePermissionAtom)
	const [, setSessionStatus] = useAtom(sessionStatusAtom)

	if (!permission) return null

	const allowOption = permission.options.find((option) =>
		option.kind.includes("allow"),
	)
	const denyOption = permission.options.find((option) =>
		option.kind.includes("reject"),
	)

	async function handleDecision(optionId: string) {
		await respondPermission(permission!.requestId, optionId)
		setPermission(null)
		setSessionStatus("generating")
	}

	return (
		<div className="mx-4 mb-3 rounded-lg border border-ring/40 bg-card p-4 shadow-lg">
			<div className="mb-3 flex items-start gap-2">
				<ShieldAlert className="mt-0.5 size-4 text-ring" />
				<div>
					<p className="text-sm font-medium">Permiso requerido</p>
					<p className="text-xs text-muted-foreground">
						El agente quiere ejecutar una acción. Aprueba o deniega antes de continuar.
					</p>
				</div>
			</div>

			<pre className="mb-3 max-h-32 overflow-auto rounded-md bg-muted/50 p-2 font-mono text-xs">
				{JSON.stringify(permission.toolCall, null, 2)}
			</pre>

			<div className="flex gap-2">
				{allowOption ? (
					<Button onClick={() => handleDecision(allowOption.optionId)}>
						Aprobar
					</Button>
				) : null}
				{denyOption ? (
					<Button
						variant="destructive"
						onClick={() => handleDecision(denyOption.optionId)}
					>
						Denegar
					</Button>
				) : null}
			</div>
		</div>
	)
}