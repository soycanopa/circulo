/**
 * Shared server configuration constants.
 *
 * Used by both the main process and the renderer. Keep this module
 * free of Electron or React imports so it can be bundled in either context.
 */

import type { LocalServerConfig, ServerSettings } from "../preload/api"

export function getDefaultLocalServerName(platform: NodeJS.Platform): string {
	if (platform === "darwin") return "This Mac"
	if (platform === "win32") return "This PC"
	return "Local"
}

/** The built-in local server entry. Always present, cannot be deleted. */
export function getDefaultLocalServer(platform: NodeJS.Platform): LocalServerConfig {
	return {
		id: "local",
		name: getDefaultLocalServerName(platform),
		type: "local",
	}
}

/** @deprecated Use getDefaultLocalServer(platform) instead. Kept for backward compat. */
export const DEFAULT_LOCAL_SERVER: LocalServerConfig = getDefaultLocalServer("darwin")

/** Default server settings for fresh installs. */
export const DEFAULT_SERVER_SETTINGS: ServerSettings = {
	servers: [DEFAULT_LOCAL_SERVER],
	activeServerId: "local",
}
