export const TERMINAL_MIN_HEIGHT = 140
export const TERMINAL_MAX_HEIGHT = 520
export const TERMINAL_DEFAULT_HEIGHT = 240

export function getDefaultShell(): { file: string; args: string[] } {
	const platform =
		typeof navigator !== "undefined" ? navigator.platform.toLowerCase() : ""

	if (platform.includes("win")) {
		return { file: "powershell.exe", args: [] }
	}

	if (platform.includes("mac")) {
		return { file: "/bin/zsh", args: ["-l"] }
	}

	return { file: "/bin/bash", args: ["-l"] }
}

export function clampTerminalHeight(height: number) {
	return Math.min(TERMINAL_MAX_HEIGHT, Math.max(TERMINAL_MIN_HEIGHT, height))
}