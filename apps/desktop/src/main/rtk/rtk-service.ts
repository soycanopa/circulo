import { execSync, spawn } from "node:child_process"
import fs from "node:fs"
import path from "node:path"
import { homedir } from "node:os"
import { createLogger } from "../logger"
import { RTK_PLUGIN_SOURCE } from "./plugin-source"

const log = createLogger("rtk-service")

// ============================================================
// Paths
// ============================================================

const OPENCODE_PLUGINS_DIR = path.join(homedir(), ".config", "opencode", "plugins")
const PLUGIN_FILE = path.join(OPENCODE_PLUGINS_DIR, "rtk.ts")

// ============================================================
// Binary detection
// ============================================================

/** Check if the `rtk` binary is installed and on PATH. */
export function isRtkInstalled(): boolean {
	try {
		execSync("which rtk", { stdio: "ignore" })
		return true
	} catch {
		return false
	}
}

/** Get the installed version of `rtk`, or null if not installed. */
export function getRtkVersion(): string | null {
	try {
		const output = execSync("rtk --version", { encoding: "utf-8", stdio: ["ignore", "pipe", "ignore"] })
		const match = output.match(/(\d+\.\d+\.\d+)/)
		return match ? match[1] : output.trim()
	} catch {
		return null
	}
}

// ============================================================
// Plugin management
// ============================================================

/** Write the RTK plugin to the OpenCode plugins directory. Returns true on success. */
export function installPlugin(): boolean {
	try {
		if (!fs.existsSync(OPENCODE_PLUGINS_DIR)) {
			fs.mkdirSync(OPENCODE_PLUGINS_DIR, { recursive: true })
		}
		fs.writeFileSync(PLUGIN_FILE, RTK_PLUGIN_SOURCE, "utf-8")
		log.info("RTK plugin installed", { path: PLUGIN_FILE })
		return true
	} catch (err) {
		log.error("Failed to install RTK plugin", err)
		return false
	}
}

/** Remove the RTK plugin from the OpenCode plugins directory. */
export function uninstallPlugin(): void {
	try {
		if (fs.existsSync(PLUGIN_FILE)) {
			fs.unlinkSync(PLUGIN_FILE)
			log.info("RTK plugin uninstalled", { path: PLUGIN_FILE })
		}
	} catch (err) {
		log.error("Failed to uninstall RTK plugin", err)
	}
}

/** Check if the RTK plugin file exists. */
export function isPluginActive(): boolean {
	return fs.existsSync(PLUGIN_FILE)
}

// ============================================================
// Terminal install helper
// ============================================================

/** Open the system terminal with the RTK install command pre-loaded. */
export function openTerminalForInstall(): void {
	const installCmd =
		'curl -fsSL https://raw.githubusercontent.com/rtk-ai/rtk/refs/heads/master/install.sh | sh'

	if (process.platform === "darwin") {
		const script = `tell application "Terminal" to do script "clear; echo 'Installing RTK...'; ${installCmd}"`
		spawn("osascript", ["-e", script], { detached: true, stdio: "ignore" }).unref()
	} else if (process.platform === "linux") {
		// Try common terminals in order
		const terminals = ["gnome-terminal", "konsole", "xterm"]
		for (const term of terminals) {
			try {
				execSync(`which ${term}`, { stdio: "ignore" })
				if (term === "gnome-terminal") {
					spawn(term, ["--", "bash", "-c", `echo 'Installing RTK...'; ${installCmd}; exec bash`], {
						detached: true,
						stdio: "ignore",
					}).unref()
				} else if (term === "konsole") {
					spawn(term, ["-e", "bash", "-c", `echo 'Installing RTK...'; ${installCmd}; exec bash`], {
						detached: true,
						stdio: "ignore",
					}).unref()
				} else {
					spawn(term, ["-e", `echo 'Installing RTK...'; ${installCmd}; exec bash`], {
						detached: true,
						stdio: "ignore",
					}).unref()
				}
				return
			} catch {
				// Try next terminal
			}
		}
		log.warn("No supported terminal found on Linux")
	}
	// Windows: open cmd.exe
	if (process.platform === "win32") {
		spawn("cmd.exe", ["/c", "start", "cmd.exe", "/k", installCmd], {
			detached: true,
			stdio: "ignore",
		}).unref()
	}
}
