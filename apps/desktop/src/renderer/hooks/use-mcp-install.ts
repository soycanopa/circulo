import type { McpStatus } from "@opencode-ai/sdk/v2/client"
import { useCallback, useEffect, useRef, useState } from "react"
import { useAtomValue } from "jotai"
import { serverConnectedAtom } from "../atoms/connection"
import { messagesFamily } from "../atoms/messages"
import { partsFamily } from "../atoms/parts"
import { sessionFamily } from "../atoms/sessions"
import { appStore } from "../atoms/store"
import { getBaseClient, getProjectClient } from "../services/connection-manager"
import { getHomeDir } from "../services/backend"
import type { McpTemplate } from "../components/settings/mcp-templates"

// ============================================================
// Types
// ============================================================

export type InstallStatus = "idle" | "installing" | "connected" | "failed"

export interface InstallState {
	status: InstallStatus
	progressMessages: string[]
	error: string | null
}

// ============================================================
// Module-level state setter (hack: async helpers need it)
// ============================================================

let _setStates: React.Dispatch<React.SetStateAction<Record<string, InstallState>>> | null = null

// ============================================================
// Internal helpers
// ============================================================

function updateInstallState(templateId: string, update: Partial<InstallState>) {
	_setStates?.((prev) => {
		const existing = prev[templateId]
		const base: InstallState = existing
			? { ...existing }
			: { status: "installing", progressMessages: [], error: null }
		return {
			...prev,
			[templateId]: { ...base, ...update },
		}
	})
}

function setProgress(templateId: string, messages: string[]) {
	updateInstallState(templateId, { progressMessages: messages, status: "installing" })
}

function setDone(templateId: string, status: "connected" | "failed", messages: string[]) {
	updateInstallState(templateId, {
		status,
		progressMessages: messages,
		error: status === "failed" ? (messages[0] ?? "Installation failed") : null,
	})
}

function waitForMcpStatus(
	serverName: string,
	timeoutMs: number,
	signal: AbortSignal,
): Promise<McpStatus> {
	return new Promise((resolve, reject) => {
		const start = Date.now()
		const check = () => {
			if (signal.aborted) return reject(new Error("Cancelled"))
			const client = getBaseClient()
			if (!client) return reject(new Error("Not connected to server"))

			client.mcp
				.status()
				.then((result) => {
					const data = (result.data ?? {}) as Record<string, McpStatus>
					const status = data[serverName]
					if (status) return resolve(status)
					if (Date.now() - start > timeoutMs) return reject(new Error("Timeout waiting for MCP server"))
					setTimeout(check, 2000)
				})
				.catch(() => {
					if (Date.now() - start > timeoutMs) return reject(new Error("Timeout waiting for MCP server"))
					setTimeout(check, 2000)
				})
		}
		check()
	})
}

function waitForSessionIdle(sessionId: string, timeoutMs: number, signal: AbortSignal): Promise<void> {
	return new Promise((resolve, reject) => {
		const start = Date.now()
		const check = () => {
			if (signal.aborted) return reject(new Error("Cancelled"))
			const entry = appStore.get(sessionFamily(sessionId))
			if (entry?.status?.type === "idle") return resolve()
			if (Date.now() - start > timeoutMs) return reject(new Error("Session timed out"))
			setTimeout(check, 1000)
		}
		check()
	})
}

function getAgentMessages(sessionId: string): string[] {
	const msgs = appStore.get(messagesFamily(sessionId))
	const lines: string[] = []
	const assistantMessages = msgs.filter((m) => m.role === "assistant").slice(-10)

	for (const msg of assistantMessages) {
		const parts = appStore.get(partsFamily(msg.id))
		for (const part of parts) {
			if (part.type === "text" && "text" in part && typeof part.text === "string") {
				const text = part.text.trim()
				if (text.length > 0 && text.length < 200) {
					lines.push(text)
				}
			}
		}
	}

	return lines
}

function pollAgentMessages(
	sessionId: string,
	signal: AbortSignal,
	onMessages: (msgs: string[]) => void,
): Promise<void> {
	return new Promise((resolve) => {
		const check = () => {
			if (signal.aborted) return resolve()
			const entry = appStore.get(sessionFamily(sessionId))
			if (!entry) return resolve()

			const messages = getAgentMessages(sessionId)
			if (messages.length > 0) onMessages(messages)

			if (entry.status?.type === "idle") return resolve()

			setTimeout(check, 2000)
		}
		check()
	})
}

// ============================================================
// Simple install (direct mcp.add())
// ============================================================

async function installSimple(
	template: McpTemplate,
	envValues: Record<string, string> | undefined,
	signal: AbortSignal,
) {
	const client = getBaseClient()
	if (!client) throw new Error("Not connected to OpenCode server")

	const config = { ...template.config } as Record<string, unknown>

	if (template.envVars && template.envVars.length > 0) {
		const env: Record<string, string> = {}
		for (const ev of template.envVars) {
			const value = envValues?.[ev.key]
			if (value) env[ev.key] = value
		}
		if (Object.keys(env).length > 0) {
			;(config as { environment?: Record<string, string> }).environment = env
		}
	}

	setProgress(template.id, ["Adding MCP server..."])
	await client.mcp.add({ name: template.id, config: config as never })

	if (signal.aborted) return

	setProgress(template.id, ["Connecting to MCP server..."])

	const status = await waitForMcpStatus(template.id, 30000, signal)

	if (status.status === "connected") {
		setDone(template.id, "connected", [`${template.name} is now connected and ready to use`])
	} else if (status.status === "failed") {
		const errorMsg = "error" in status ? status.error : "Connection failed"
		setDone(template.id, "failed", [errorMsg])
	} else {
		setDone(template.id, "failed", [`Unexpected status: ${status.status}`])
	}
}

// ============================================================
// Agent install (background session)
// ============================================================

async function installAgent(template: McpTemplate, signal: AbortSignal) {
	const homeDir = await getHomeDir()
	if (!homeDir) throw new Error("Could not determine home directory")
	const client = getProjectClient(homeDir)
	if (!client) throw new Error("Not connected to OpenCode server")

	setProgress(template.id, ["Creating install session..."])

	const result = await client.session.create({
		title: `Installing ${template.name}`,
		agent: "build",
	})

	const session = result.data
	if (!session?.id) throw new Error("Failed to create install session")
	const sessionId = session.id

	if (signal.aborted) return

	// Start polling for progress messages in parallel
	const progressPromise = pollAgentMessages(sessionId, signal, (messages) => {
		setProgress(template.id, messages)
	})

	// Send the prompt (fire and forget)
	try {
		await client.session.promptAsync({
			sessionID: sessionId,
			agent: "build",
			parts: [{ type: "text", text: template.prompt ?? `Install ${template.name}` }],
		})
	} catch (err) {
		const msg = err instanceof Error ? err.message : "Failed to send prompt"
		if (!signal.aborted) setProgress(template.id, [`Error: ${msg}`])
		return
	}

	if (signal.aborted) return

	// Wait for session to complete
	try {
		await waitForSessionIdle(sessionId, 120000, signal)
	} catch (err) {
		if (signal.aborted) return
		try { await client.session.delete({ sessionID: sessionId }) } catch { /* ignore */ }
		setDone(template.id, "failed", [err instanceof Error ? err.message : "Install session timed out"])
		return
	}

	// Stop progress polling
	await progressPromise

	// Clean up the install session
	try { await client.session.delete({ sessionID: sessionId }) } catch { /* ignore */ }

	// Check if MCP server appears
	const baseClient = getBaseClient()
	if (baseClient && !signal.aborted) {
		try {
			const statusResult = await baseClient.mcp.status()
			const data = (statusResult.data ?? {}) as Record<string, McpStatus>
			const serverStatus = data[template.id]
			if (serverStatus) {
				const resolvedStatus: "connected" | "failed" = serverStatus.status === "connected"
					? "connected" : "failed"
				setDone(template.id, resolvedStatus, [
					`${template.name} has been installed and is ${serverStatus.status}`,
				])
				return
			}
		} catch { /* ignore */ }
	}

	setDone(template.id, "connected", [
		"Installation complete",
		"Check Configured Servers above to enable it",
	])
}

// ============================================================
// Hook
// ============================================================

export function useMcpInstall() {
	const connected = useAtomValue(serverConnectedAtom)
	const [installStates, setInstallStates] = useState<Record<string, InstallState>>({})
	const abortRef = useRef<Record<string, AbortController>>({})

	// Keep module-level setter in sync
	_setStates = setInstallStates

	// Cleanup on unmount
	useEffect(() => {
		return () => {
			_setStates = null
			for (const ctrl of Object.values(abortRef.current)) {
				ctrl.abort()
			}
		}
	}, [])

	const getState = useCallback(
		(templateId: string): InstallState => {
			return installStates[templateId] ?? { status: "idle", progressMessages: [], error: null }
		},
		[installStates],
	)

	const install = useCallback(
		async (template: McpTemplate, envValues?: Record<string, string>) => {
			if (!connected) {
				setDone(template.id, "failed", ["OpenCode server is offline"])
				return
			}

			const abort = new AbortController()
			abortRef.current[template.id] = abort

			updateInstallState(template.id, { status: "installing", error: null })

			try {
				if (template.type === "simple") {
					await installSimple(template, envValues, abort.signal)
				} else {
					await installAgent(template, abort.signal)
				}
			} catch (err) {
				if (abort.signal.aborted) return
				setDone(template.id, "failed", [
					err instanceof Error ? err.message : "Installation failed",
				])
			} finally {
				delete abortRef.current[template.id]
			}
		},
		[connected],
	)

	const cancel = useCallback((templateId: string) => {
		abortRef.current[templateId]?.abort()
		updateInstallState(templateId, { status: "idle", error: null })
	}, [])

	return { install, cancel, getState }
}
