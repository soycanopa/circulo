import { useAtomValue, useSetAtom } from "jotai"
import { useEffect, useRef } from "react"
import {
	findAutoApproveConfigOption,
	isAutoApproveConfigEnabled,
	matchesAllowedPattern,
	permissionToolName,
	pickAutoApprovePermissionOption,
} from "@/lib/auto-approve"
import { respondPermission } from "@/lib/tauri"
import {
	activePermissionAtom,
	appSettingsAtom,
	errorMessageAtom,
	pendingPermissionsAtom,
	sessionStatusAtom,
	visibleConfigOptionsAtom,
} from "@/stores/atoms"

/** Auto-approve tool permissions when the user enabled unattended edits. */
export function useAutoApprove() {
	const permission = useAtomValue(activePermissionAtom)
	const queue = useAtomValue(pendingPermissionsAtom)
	const appSettings = useAtomValue(appSettingsAtom)
	const configOptions = useAtomValue(visibleConfigOptionsAtom)
	const setPermission = useSetAtom(activePermissionAtom)
	const setQueue = useSetAtom(pendingPermissionsAtom)
	const setStatus = useSetAtom(sessionStatusAtom)
	const setError = useSetAtom(errorMessageAtom)
	const handling = useRef<string | null>(null)

	const configOption = findAutoApproveConfigOption(configOptions)
	const enabled = configOption
		? isAutoApproveConfigEnabled(configOption)
		: (appSettings?.autoApproveEnabled ?? false)

	useEffect(() => {
		if (!permission) return

		// A remembered "allow always" pattern overrides the global toggle:
		// it should be honored even when unattended edits are off.
		const remembered = matchesAllowedPattern(
			permissionToolName(permission),
			appSettings?.allowedToolPatterns ?? [],
		)
		if (!enabled && !remembered) return
		if (handling.current === permission.requestId) return

		const optionId = pickAutoApprovePermissionOption(permission.options)
		if (!optionId) return

		handling.current = permission.requestId
		void respondPermission(permission.requestId, optionId, permission.sessionId)
			.then(() => {
				const next = queue.slice(1)
				setQueue(next)
				setPermission(next[0] ?? null)
				setStatus(next.length === 0 ? "generating" : "awaiting_permission")
			})
			.catch((err: unknown) => {
				setError(
					err instanceof Error
						? err.message
						: "Failed to auto-approve permission",
				)
			})
			.finally(() => {
				if (handling.current === permission.requestId) {
					handling.current = null
				}
			})
	}, [
		enabled,
		permission,
		queue,
		appSettings?.allowedToolPatterns,
		setError,
		setPermission,
		setQueue,
		setStatus,
	])
}
