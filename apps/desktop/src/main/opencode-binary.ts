import { execFile } from "node:child_process"
import { homedir } from "node:os"
import path from "node:path"
import { getOpencodeBinary } from "./settings-store"

const log = console

/**
 * Resolves the opencode binary to use.
 *
 * Priority:
 * 1. User-configured custom path (from settings)
 * 2. `opencode` on PATH (with ~/.opencode/bin prepended)
 * 3. `opencode-cli` as fallback binary name
 */
export function resolveOpencodeBinary(): string {
	const configured = getOpencodeBinary()
	if (configured) return configured

	return getDefaultOpencodeBinaryName()
}

/**
 * Resolves the opencode binary path for use with execFile/which.
 * Includes the default binary name after PATH augmentation.
 */
export function getDefaultOpencodeBinaryName(): string {
	return "opencode"
}

/**
 * Augmented PATH with ~/.opencode/bin prepended.
 * Use this as the `env.PATH` when spawning child processes that need to find opencode.
 */
export function getAugmentedPath(): string {
	const opencodeBinDir = path.join(homedir(), ".opencode", "bin")
	const sep = process.platform === "win32" ? ";" : ":"
	return `${opencodeBinDir}${sep}${process.env.PATH ?? ""}`
}

/**
 * Attempts to find the opencode binary on the system PATH.
 * Tries `opencode` first, then `opencode-cli` as fallback.
 * Returns the resolved binary name, or `"opencode"` as default.
 */
export async function detectOpencodeBinary(): Promise<string> {
	const configured = getOpencodeBinary()
	if (configured) return configured

	const candidates = ["opencode", "opencode-cli"]
	const whichCmd = process.platform === "win32" ? "where" : "which"
	const env = { ...process.env, PATH: getAugmentedPath() }

	for (const candidate of candidates) {
		try {
			await execFileAsync(whichCmd, [candidate], { env })
			return candidate
		} catch {
			// Not found, try next candidate
		}
	}

	return "opencode"
}

function execFileAsync(
	command: string,
	args: string[],
	options: { env: Record<string, string | undefined> },
): Promise<string> {
	return new Promise((resolve, reject) => {
		execFile(command, args, options, (err, stdout) => {
			if (err) reject(err)
			else resolve(stdout.trim())
		})
	})
}
