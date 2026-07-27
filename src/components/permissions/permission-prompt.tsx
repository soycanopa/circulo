import { useAtom } from "jotai"
import { respondPermission } from "@/lib/tauri"
import { activePermissionAtom, sessionStatusAtom } from "@/stores/atoms"

export function PermissionPrompt() {
	const [permission, setPermission] = useAtom(activePermissionAtom)
	const [, setStatus] = useAtom(sessionStatusAtom)

	if (!permission) return null

	async function respond(optionId: string) {
		if (!permission) return
		try {
			await respondPermission(permission.requestId, optionId)
			setPermission(null)
			setStatus("generating")
		} catch {
			// keep card visible
		}
	}

	return (
		<div className="mb-2 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2">
			<p className="text-xs font-medium text-amber-100">Permission required</p>
			<p className="mt-0.5 text-[11px] text-muted">
				The agent wants to run a tool. Choose an option to continue.
			</p>
			<div className="mt-2 flex flex-wrap gap-1.5">
				{permission.options.map((option) => (
					<button
						key={option.optionId}
						type="button"
						onClick={() => void respond(option.optionId)}
						className="rounded-md border border-border bg-surface px-2.5 py-1 text-xs text-fg transition hover:bg-white/5"
					>
						{option.name || option.optionId}
					</button>
				))}
			</div>
		</div>
	)
}
