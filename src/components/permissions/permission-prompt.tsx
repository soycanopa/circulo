import { useAtom, useSetAtom } from "jotai"
import { respondPermission } from "@/lib/tauri"
import {
	activePermissionAtom,
	errorMessageAtom,
	pendingPermissionsAtom,
	sessionStatusAtom,
} from "@/stores/atoms"

export function PermissionPrompt() {
	const [permission, setPermission] = useAtom(activePermissionAtom)
	const [queue, setQueue] = useAtom(pendingPermissionsAtom)
	const setStatus = useSetAtom(sessionStatusAtom)
	const setError = useSetAtom(errorMessageAtom)

	if (!permission) return null

	async function respond(optionId: string) {
		if (!permission) return
		if (!permission.options.some((o) => o.optionId === optionId)) {
			setError(`Invalid option id: ${optionId}`)
			return
		}
		try {
			await respondPermission(permission.requestId, optionId, permission.sessionId)
			const next = queue.slice(1)
			setQueue(next)
			setPermission(next[0] ?? null)
			setStatus(next.length === 0 ? "generating" : "awaiting_permission")
		} catch (err) {
			// keep card visible — Rust may have already cancelled the waiter.
			setError(
				err instanceof Error ? err.message : "Failed to respond to permission",
			)
		}
	}

	return (
		<div className="mb-2 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2">
			<p className="text-xs font-medium text-amber-100">
				Permission required
				{queue.length > 1
					? ` (1 of ${queue.length})`
					: ""}
			</p>
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
