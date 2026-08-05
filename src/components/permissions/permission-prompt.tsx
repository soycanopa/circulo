import { useAtom, useAtomValue, useSetAtom } from "jotai"
import { BookmarkPlus } from "lucide-react"
import {
	isAllowPermissionOption,
	permissionToolName,
} from "@/lib/auto-approve"
import { respondPermission, setAllowedTool } from "@/lib/tauri"
import {
	activePermissionAtom,
	appSettingsAtom,
	errorMessageAtom,
	pendingPermissionsAtom,
	sessionStatusAtom,
} from "@/stores/atoms"

export function PermissionPrompt() {
	const [permission, setPermission] = useAtom(activePermissionAtom)
	const [queue, setQueue] = useAtom(pendingPermissionsAtom)
	const appSettings = useAtomValue(appSettingsAtom)
	const setStatus = useSetAtom(sessionStatusAtom)
	const setError = useSetAtom(errorMessageAtom)

	if (!permission) return null

	const toolName = permissionToolName(permission)

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

	async function respondAndRemember(optionId: string, pattern: string) {
		if (!permission) return
		try {
			await setAllowedTool(pattern, true)
		} catch (err) {
			setError(
				err instanceof Error ? err.message : "Failed to remember tool",
			)
			return
		}
		await respond(optionId)
	}

	const remembered = new Set(appSettings?.allowedToolPatterns ?? [])

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
				{permission.options.map((option) => {
					const canRemember = isAllowPermissionOption(option) && Boolean(toolName)
					const alreadyRemembered = canRemember && remembered.has(toolName)
					return (
						<div
							key={option.optionId}
							className="flex items-center gap-1 rounded-md border border-border bg-surface px-1.5 py-1"
						>
							<button
								type="button"
								onClick={() => void respond(option.optionId)}
								className="rounded px-2 py-1 text-xs text-fg transition hover:bg-white/5"
							>
								{option.name || option.optionId}
							</button>
							{canRemember ? (
								<button
									type="button"
									disabled={alreadyRemembered}
									onClick={() =>
										void respondAndRemember(option.optionId, toolName)
									}
									title={
										alreadyRemembered
											? "This tool is already allowed always"
											: "Always allow this tool without asking"
									}
									className="rounded px-1 py-1 text-muted transition hover:bg-white/5 hover:text-amber-200 disabled:cursor-default disabled:opacity-40"
								>
									<BookmarkPlus className="size-3.5" />
								</button>
							) : null}
						</div>
					)
				})}
			</div>
		</div>
	)
}
