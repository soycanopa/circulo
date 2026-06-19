import type { McpStatus } from "@opencode-ai/sdk/v2/client"
import { Badge } from "@circulo/ui/components/badge"
import { Button } from "@circulo/ui/components/button"
import { Skeleton } from "@circulo/ui/components/skeleton"
import { Switch } from "@circulo/ui/components/switch"
import { useNavigate } from "@tanstack/react-router"
import { BlocksIcon, ExternalLinkIcon, RefreshCwIcon } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"
import { serverConnectedAtom } from "../../atoms/connection"
import { useAtomValue, useSetAtom } from "jotai"
import { chatInitDirectoryAtom } from "../../atoms/chat"
import { useMcpOAuth, useMcpStatus, useMcpToggle } from "../../hooks/use-opencode-data"
import { fetchOpenCodeUrl, getHomeDir } from "../../services/backend"
import { SettingsRow } from "./settings-row"
import { SettingsSection } from "./settings-section"

// ============================================================
// Helpers
// ============================================================

function getStatusVariant(status: string): "default" | "secondary" | "destructive" | "outline" {
	switch (status) {
		case "connected":
			return "default"
		case "disabled":
			return "secondary"
		case "failed":
			return "destructive"
		case "needs_auth":
		case "needs_client_registration":
			return "outline"
		default:
			return "secondary"
	}
}

function getStatusLabel(status: string): string {
	switch (status) {
		case "connected":
			return "Connected"
		case "disabled":
			return "Disabled"
		case "failed":
			return "Failed"
		case "needs_auth":
			return "Needs Auth"
		case "needs_client_registration":
			return "Needs Registration"
		default:
			return status
	}
}

function getStatusDotClass(status: string): string {
	switch (status) {
		case "connected":
			return "bg-green-500"
		case "disabled":
			return "bg-gray-400"
		case "failed":
			return "bg-red-500"
		case "needs_auth":
		case "needs_client_registration":
			return "bg-amber-500"
		default:
			return "bg-gray-400"
	}
}

// ============================================================
// McpServerRow
// ============================================================

function McpServerRow({
	name,
	status,
	disabled: isRowDisabled,
	onToggle,
	onStartAuth,
	onAuthenticate,
}: {
	name: string
	status: McpStatus
	disabled?: boolean
	onToggle: (enabled: boolean) => void
	onStartAuth: () => void
	onAuthenticate: () => void
}) {
	const isEnabled = status.status !== "disabled"
	const isFailed = status.status === "failed"
	const needsAuth = status.status === "needs_auth"
	const needsRegistration = status.status === "needs_client_registration"
	const errorMessage = "error" in status ? status.error : null

	return (
		<div className="space-y-2">
			<SettingsRow label={name} htmlFor={`mcp-toggle-${name}`}>
				<div className="flex items-center gap-2">
					<div className="flex items-center gap-1.5">
						<span
							className={`inline-block size-2 rounded-full ${getStatusDotClass(status.status)}`}
							aria-hidden="true"
						/>
						<Badge variant={getStatusVariant(status.status)}>
							{getStatusLabel(status.status)}
						</Badge>
					</div>
					<Switch
						id={`mcp-toggle-${name}`}
						checked={isEnabled}
						onCheckedChange={onToggle}
						disabled={isRowDisabled}
					/>
				</div>
			</SettingsRow>

			{isFailed && errorMessage && (
				<p className="px-4 text-xs text-destructive">{errorMessage}</p>
			)}

			<div className="flex items-center gap-2 px-4">
				{needsAuth && (
					<Button
						variant="outline"
						size="sm"
						onClick={onStartAuth}
						disabled={isRowDisabled}
					>
						<ExternalLinkIcon aria-hidden="true" className="mr-1 size-3" />
						Authenticate
					</Button>
				)}
				{needsRegistration && (
					<Button
						variant="outline"
						size="sm"
						onClick={onAuthenticate}
						disabled={isRowDisabled}
					>
						Register Client
					</Button>
				)}
			</div>
		</div>
	)
}

// ============================================================
// McpSettings page
// ============================================================

export function McpSettings() {
	const navigate = useNavigate()
	const connected = useAtomValue(serverConnectedAtom)
	const { data, loading, error, reload } = useMcpStatus()
	const { toggleServer } = useMcpToggle()
	const { startAuth, authenticate } = useMcpOAuth()
	const setChatInitDir = useSetAtom(chatInitDirectoryAtom)
	const [startingServer, setStartingServer] = useState(false)
	const [togglingServers, setTogglingServers] = useState<Set<string>>(new Set())

	const handleStartServer = async () => {
		setStartingServer(true)
		try {
			await fetchOpenCodeUrl()
		} catch (err) {
			toast.error("Failed to start OpenCode server")
		} finally {
			setStartingServer(false)
		}
	}

	const handleToggle = async (name: string, enabled: boolean) => {
		setTogglingServers((prev) => new Set(prev).add(name))
		try {
			await toggleServer(name, enabled)
		} catch (err) {
			toast.error(
				err instanceof Error ? err.message : `Failed to ${enabled ? "enable" : "disable"} server`,
			)
		} finally {
			setTogglingServers((prev) => {
				const next = new Set(prev)
				next.delete(name)
				return next
			})
		}
	}

	const handleStartAuth = async (name: string) => {
		try {
			await startAuth(name)
			toast.success(`OAuth flow started for ${name}`)
		} catch (err) {
			toast.error(
				err instanceof Error ? err.message : `Failed to start OAuth for ${name}`,
			)
		}
	}

	const handleAuthenticate = async (name: string) => {
		try {
			await authenticate(name)
			toast.success(`Client registration completed for ${name}`)
		} catch (err) {
			toast.error(
				err instanceof Error ? err.message : `Failed to register client for ${name}`,
			)
		}
	}

	const handleAddServer = async () => {
		const homeDir = await getHomeDir()
		setChatInitDir(homeDir)
		navigate({
			to: "/",
			search: { mcpSetup: "true" } as Record<string, string>,
		})
	}

	const entries = data ? Object.entries(data) : []
	const anyToggling = togglingServers.size > 0

	// --- Offline state
	if (!connected) {
		return (
			<div className="space-y-8">
				<div>
					<h2 className="text-xl font-semibold">MCP Servers</h2>
					<p className="mt-1 text-sm text-muted-foreground">
						Manage MCP servers that extend agent capabilities.
					</p>
				</div>
				<div className="flex flex-col items-center justify-center rounded-lg border py-12 text-center">
					<BlocksIcon aria-hidden="true" className="mb-3 size-10 text-muted-foreground" />
					<p className="text-sm font-medium">OpenCode server is offline</p>
					<p className="mt-1 text-xs text-muted-foreground">
						Start the OpenCode server to manage MCP servers.
					</p>
					<Button
						variant="outline"
						size="sm"
						className="mt-4"
						onClick={handleStartServer}
						disabled={startingServer}
					>
						{startingServer ? "Starting..." : "Start Server"}
					</Button>
				</div>
			</div>
		)
	}

	// --- Error state
	if (error) {
		return (
			<div className="space-y-8">
				<div>
					<h2 className="text-xl font-semibold">MCP Servers</h2>
					<p className="mt-1 text-sm text-muted-foreground">
						Manage MCP servers that extend agent capabilities.
					</p>
				</div>
				<div className="flex flex-col items-center justify-center rounded-lg border py-12 text-center">
					<p className="text-sm text-destructive">Failed to load MCP servers</p>
					<p className="mt-1 text-xs text-muted-foreground">{error}</p>
					<Button variant="outline" size="sm" className="mt-4" onClick={() => reload()}>
						<RefreshCwIcon aria-hidden="true" className="mr-1 size-3" />
						Retry
					</Button>
				</div>
			</div>
		)
	}

	return (
		<div className="space-y-8">
			<div className="flex items-start justify-between">
				<div>
					<h2 className="text-xl font-semibold">MCP Servers</h2>
					<p className="mt-1 text-sm text-muted-foreground">
						Manage Model Context Protocol (MCP) servers that extend agent capabilities with
						external tools and resources.
					</p>
				</div>
				<Button size="sm" onClick={handleAddServer}>
					<BlocksIcon aria-hidden="true" className="mr-1.5 size-4" />
					Add MCP Server
				</Button>
			</div>

			<SettingsSection title="Configured Servers">
				{loading && entries.length === 0 ? (
					<div className="space-y-4 p-4">
						<Skeleton className="h-10 w-full" />
						<Skeleton className="h-10 w-full" />
						<Skeleton className="h-10 w-full" />
					</div>
				) : entries.length === 0 ? (
					<div className="flex flex-col items-center justify-center py-12 text-center">
						<BlocksIcon aria-hidden="true" className="mb-3 size-10 text-muted-foreground" />
						<p className="text-sm text-muted-foreground">No MCP servers configured.</p>
						<p className="mt-1 text-xs text-muted-foreground">
							Add an MCP server to extend your agents with custom tools, resources, and
							prompts.
						</p>
						<Button variant="outline" size="sm" className="mt-4" onClick={handleAddServer}>
							<BlocksIcon aria-hidden="true" className="mr-1.5 size-4" />
							Add MCP Server
						</Button>
					</div>
				) : (
					<div className="divide-y">
						{entries.map(([name, status]) => (
							<div key={name} className="py-3 first:pt-0 last:pb-0">
								<McpServerRow
									name={name}
									status={status}
									disabled={anyToggling}
									onToggle={(enabled) => handleToggle(name, enabled)}
									onStartAuth={() => handleStartAuth(name)}
									onAuthenticate={() => handleAuthenticate(name)}
								/>
							</div>
						))}
					</div>
				)}
			</SettingsSection>
		</div>
	)
}
