import type { McpStatus } from "@opencode-ai/sdk/v2/client"
import { toast } from "sonner"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { useAtomValue } from "jotai"
import { serverConnectedAtom } from "../atoms/connection"
import { sessionFamily } from "../atoms/sessions"
import { appStore } from "../atoms/store"
import { getBaseClient, getProjectClient } from "../services/connection-manager"
import { getHomeDir } from "../services/backend"
import type { McpTemplate } from "../components/settings/mcp-templates"

// ============================================================
// Types
// ============================================================

export type InstallStatus = "idle" | "installing" | "connected" | "failed"

/** Sub-state while an install is in progress. */
export type InstallPhase = "running" | "verifying"

export interface InstallState {
	status: InstallStatus
	/** Only meaningful while `status === "installing"`. */
	phase?: InstallPhase
	progressMessages: string[]
	error: string | null
}

// ============================================================
// Pure helpers
// ============================================================

/**
 * Verify that an MCP server has been installed and is connected.
 */
async function verifyMcpInstalled(templateId: string): Promise<boolean> {
	const client = getBaseClient()
	if (!client) return false

	try {
		const result = await client.mcp.status()
		const data = (result.data ?? {}) as Record<string, McpStatus>
		const status = data[templateId]
		return status?.status === "connected"
	} catch {
		return false
	}
}



// ============================================================
// Install context (state setters + notify, bound per hook instance)
// ============================================================

type InstallStatesSetter = React.Dispatch<React.SetStateAction<Record<string, InstallState>>>

interface InstallCtx {
	setProgress: (templateId: string, messages: string[], phase?: InstallPhase) => void
	setDone: (templateId: string, status: "connected" | "failed", messages: string[]) => void
	notify: (template: McpTemplate, status: "connected" | "failed", detail?: string) => void
}

function createInstallCtx(setStates: InstallStatesSetter): InstallCtx {
	const updateInstallState = (templateId: string, update: Partial<InstallState>) => {
		setStates((prev) => {
			const existing = prev[templateId]
			const base: InstallState = existing
				? { ...existing }
				: { status: "installing", phase: "running", progressMessages: [], error: null }
			return {
				...prev,
				[templateId]: { ...base, ...update },
			}
		})
	}

	const setProgress: InstallCtx["setProgress"] = (templateId, messages, phase = "running") => {
		updateInstallState(templateId, { status: "installing", phase, progressMessages: messages })
	}

	const setDone: InstallCtx["setDone"] = (templateId, status, messages) => {
		updateInstallState(templateId, {
			status,
			phase: undefined,
			progressMessages: messages,
			error: status === "failed" ? (messages[0] ?? "Installation failed") : null,
		})
	}

	const notify: InstallCtx["notify"] = (template, status, detail) => {
		if (status === "connected") {
			toast.success(`${template.name} installed`)
		} else {
			toast.error(`Failed to install ${template.name}`, { description: detail })
		}
	}

	return { setProgress, setDone, notify }
}

// ============================================================
// Simple install (creates visible session)
// ============================================================

async function installSimple(
	template: McpTemplate,
	envValues: Record<string, string> | undefined,
	signal: AbortSignal,
	ctx: InstallCtx,
) {
	const homeDir = await getHomeDir()
	if (!homeDir) throw new Error("Could not determine home directory")
	const client = getProjectClient(homeDir)
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

	ctx.setProgress(template.id, ["Creating install session..."])

	const result = await client.session.create({
		title: `Installing ${template.name} MCP`,
		agent: "build",
		permission: [
			{ permission: "bash", pattern: "npm install*", action: "allow" },
			{ permission: "bash", pattern: "npm *", action: "allow" },
			{ permission: "bash", pattern: "npx *", action: "allow" },
			{ permission: "bash", pattern: "bunx *", action: "allow" },
			{ permission: "bash", pattern: "bun *", action: "allow" },
			{ permission: "bash", pattern: "uvx *", action: "allow" },
			{ permission: "bash", pattern: "pip *", action: "allow" },
			{ permission: "bash", pattern: "pip3 *", action: "allow" },
			{ permission: "edit", pattern: "**/opencode.json", action: "allow" },
			{ permission: "edit", pattern: "**/.mcp.json", action: "allow" },
			{ permission: "edit", pattern: "**/.config/opencode/*", action: "allow" },
			{ permission: "webfetch", pattern: "*", action: "allow" },
			{ permission: "websearch", pattern: "*", action: "allow" },
		],
	})

	const session = result.data
	if (!session?.id) throw new Error("Failed to create install session")
	const sessionId = session.id

	if (signal.aborted) return

	// Send installation prompt with documentation
	try {
		const promptText = template.documentation || template.prompt || `Install ${template.name} MCP`
		await client.session.promptAsync({
			sessionID: sessionId,
			agent: "build",
			parts: [{ type: "text", text: promptText }],
		})
	} catch (err) {
		if (signal.aborted) return
		const msg = err instanceof Error ? err.message : "Failed to send prompt"
		try {
			await client.session.delete({ sessionID: sessionId })
		} catch {
			/* ignore */
		}
		ctx.setDone(template.id, "failed", [msg])
		ctx.notify(template, "failed", msg)
		return
	}

	if (signal.aborted) return

	// Wait for session to complete (120s timeout)
	try {
		await new Promise<void>((resolve, reject) => {
			const start = Date.now()
			const check = () => {
				if (signal.aborted) return reject(new Error("Cancelled"))
				const entry = appStore.get(sessionFamily(sessionId))
				if (entry?.status?.type === "idle") return resolve()
				if (Date.now() - start > 120000) return reject(new Error("Install session timed out"))
				setTimeout(check, 1000)
			}
			check()
		})
	} catch (err) {
		if (signal.aborted) return
		const msg = err instanceof Error ? err.message : "Install session timed out"
		ctx.setDone(template.id, "failed", [msg])
		ctx.notify(template, "failed", msg)
		return
	}

	if (signal.aborted) return

	// Verify installation
	ctx.setProgress(template.id, ["Verifying installation..."], "verifying")

	const isInstalled = await verifyMcpInstalled(template.id)

	if (isInstalled) {
		ctx.setDone(template.id, "connected", [
			`${template.name} has been successfully installed and is ready to use.`,
		])
		ctx.notify(template, "connected")
	} else {
		const msg = "The installation completed but the MCP server is not connected. Check the session for details."
		ctx.setDone(template.id, "failed", [msg])
		ctx.notify(template, "failed", msg)
	}
}

// ============================================================
// Hook
// ============================================================

export function useMcpInstall() {
	const connected = useAtomValue(serverConnectedAtom)
	const [installStates, setInstallStates] = useState<Record<string, InstallState>>({})
	const abortRef = useRef<Record<string, AbortController>>({})

	const ctx = useMemo(() => createInstallCtx(setInstallStates), [])

	// Abort in-flight installs on unmount.
	useEffect(() => {
		return () => {
			for (const ctrl of Object.values(abortRef.current)) {
				ctrl.abort()
			}
		}
	}, [])

	const getState = useCallback(
		(templateId: string): InstallState => {
			return (
				installStates[templateId] ?? {
					status: "idle",
					phase: undefined,
					progressMessages: [],
					error: null,
				}
			)
		},
		[installStates],
	)

	const install = useCallback(
		async (template: McpTemplate, envValues?: Record<string, string>) => {
			if (!connected) {
				ctx.setDone(template.id, "failed", ["OpenCode server is offline"])
				ctx.notify(template, "failed", "OpenCode server is offline")
				return
			}

			const abort = new AbortController()
			abortRef.current[template.id] = abort

			ctx.setProgress(template.id, [])

			try {
				await installSimple(template, envValues, abort.signal, ctx)
			} catch (err) {
				if (abort.signal.aborted) return
				const msg = err instanceof Error ? err.message : "Installation failed"
				ctx.setDone(template.id, "failed", [msg])
				ctx.notify(template, "failed", msg)
			} finally {
				delete abortRef.current[template.id]
			}
		},
		[connected, ctx],
	)

	const cancel = useCallback(
		(templateId: string) => {
			abortRef.current[templateId]?.abort()
			setInstallStates((prev) => ({
				...prev,
				[templateId]: {
					status: "idle",
					phase: undefined,
					progressMessages: [],
					error: null,
				},
			}))
		},
		[],
	)

	return { install, cancel, getState }
}
