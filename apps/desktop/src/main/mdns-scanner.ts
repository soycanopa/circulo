/**
 * mDNS service scanner for discovering OpenCode servers on the local network.
 *
 * Uses `bonjour-service` to browse for `_http._tcp` services with names
 * matching the `opencode-*` pattern. Discovered services are pushed to
 * renderer windows via IPC events.
 *
 * Lifecycle:
 * - `startMdnsScanner()` begins browsing and pushing updates
 * - `stopMdnsScanner()` tears down the browser and cleans up
 * - `getDiscoveredServers()` returns the current snapshot
 */

import { BrowserWindow } from "electron"
import { createLogger } from "./logger"

const log = createLogger("mdns")

/** Shape of a discovered mDNS service, sent to the renderer via IPC. */
export interface DiscoveredMdnsServer {
	/** Unique key derived from host:port. */
	id: string
	/** Service name from mDNS (e.g. "opencode-4096"). */
	name: string
	/** Resolved hostname or IP address. */
	host: string
	/** Port the OpenCode server is listening on. */
	port: number
	/** IP addresses reported by the service. */
	addresses: string[]
}

/** Currently known services, keyed by id. */
const discovered = new Map<string, DiscoveredMdnsServer>()

// Dynamic import handle for bonjour-service (avoids top-level require issues)
let bonjourInstance: import("bonjour-service").Bonjour | undefined
let browser: import("bonjour-service").Browser | undefined

function makeId(host: string, port: number): string {
	return `mdns-${host}:${port}`
}

function broadcastToRenderers(): void {
	const servers = Array.from(discovered.values())
	for (const win of BrowserWindow.getAllWindows()) {
		win.webContents.send("mdns:servers-changed", servers)
	}
}

/**
 * Start scanning for OpenCode servers via mDNS.
 * Safe to call multiple times; subsequent calls are no-ops.
 */
export async function startMdnsScanner(): Promise<void> {
	if (bonjourInstance) return

	try {
		const { Bonjour } = await import("bonjour-service")
		bonjourInstance = new Bonjour()

		browser = bonjourInstance.find({ type: "http" }, (service) => {
			// Only interested in opencode services (published as "opencode-{port}")
			if (!service.name.startsWith("opencode-")) return

			const host = service.host || "opencode.local"
			const port = service.port
			const id = makeId(host, port)

			if (discovered.has(id)) return

			const entry: DiscoveredMdnsServer = {
				id,
				name: service.name,
				host,
				port,
				addresses: service.addresses ?? [],
			}

			discovered.set(id, entry)
			log.info("Discovered OpenCode server", { name: service.name, host, port })
			broadcastToRenderers()
		})

		// Also listen for service removals (goodbye packets)
		browser.on("down" as string, (service: { name: string; host: string; port: number }) => {
			if (!service.name.startsWith("opencode-")) return
			const host = service.host || "opencode.local"
			const id = makeId(host, service.port)
			if (discovered.delete(id)) {
				log.info("OpenCode server went away", { name: service.name, host, port: service.port })
				broadcastToRenderers()
			}
		})

		log.info("mDNS scanner started")
	} catch (err) {
		log.error("Failed to start mDNS scanner", err)
		bonjourInstance = undefined
		browser = undefined
	}
}

/**
 * Stop the mDNS scanner and clean up resources.
 */
export function stopMdnsScanner(): void {
	if (browser) {
		try {
			browser.stop()
		} catch {
			// Ignore cleanup errors
		}
		browser = undefined
	}

	if (bonjourInstance) {
		try {
			bonjourInstance.destroy()
		} catch {
			// Ignore cleanup errors
		}
		bonjourInstance = undefined
	}

	discovered.clear()
	log.info("mDNS scanner stopped")
}

/**
 * Returns a snapshot of currently discovered servers.
 */
export function getDiscoveredServers(): DiscoveredMdnsServer[] {
	return Array.from(discovered.values())
}
