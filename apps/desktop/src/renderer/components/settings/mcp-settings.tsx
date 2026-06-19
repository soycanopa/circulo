import type { McpServerInfo } from "../../hooks/use-opencode-data"
import type { McpStatus } from "@opencode-ai/sdk/v2/client"
import { Button } from "@circulo/ui/components/button"
import { Skeleton } from "@circulo/ui/components/skeleton"
import { Switch } from "@circulo/ui/components/switch"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@circulo/ui/components/tabs"
import { Tooltip, TooltipContent, TooltipTrigger } from "@circulo/ui/components/tooltip"
import { useNavigate } from "@tanstack/react-router"
import { BlocksIcon, ChevronLeftIcon, ChevronRightIcon, KeyRoundIcon, MonitorIcon, RefreshCwIcon } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"
import { serverConnectedAtom } from "../../atoms/connection"
import { useAtomValue, useSetAtom } from "jotai"
import { chatInitDirectoryAtom } from "../../atoms/chat"
import { useMcpOAuth, useMcpStatus, useMcpToggle } from "../../hooks/use-opencode-data"
import { useMcpInstall } from "../../hooks/use-mcp-install"
import { fetchOpenCodeUrl, getHomeDir } from "../../services/backend"
import { McpTemplateCard } from "./mcp-template-card"
import { isTemplateInstalled, MCP_TEMPLATES } from "./mcp-templates"
import { SettingsRow } from "./settings-row"
import { SettingsSection } from "./settings-section"

// ============================================================
// Helpers
// ============================================================

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

function getStatusLabel(status: string): string {
	switch (status) {
		case "connected":
			return "Connected"
		case "disabled":
			return "Disabled"
		case "failed":
			return "Failed"
		case "needs_auth":
			return "Needs Authentication"
		case "needs_client_registration":
			return "Needs Client Registration"
		default:
			return status
	}
}

// ============================================================
// StatusIndicator — MonitorIcon + dot + Tooltip
// ============================================================

function StatusIndicator({ status }: { status: McpStatus }) {
	const dotClass = getStatusDotClass(status.status)
	const label = getStatusLabel(status.status)
	const errorMessage = "error" in status ? (status as { error: string }).error : null
	const isConnected = status.status === "connected"

	return (
		<Tooltip>
			<TooltipTrigger
				render={
					<span className="inline-flex items-center gap-1.5 shrink-0 cursor-default" />
				}
			>
				{isConnected ? (
					<span className="relative inline-flex size-2.5 shrink-0">
						<span className="absolute inset-0 rounded-full bg-green-500 animate-ping opacity-70" />
						<span className="relative size-2.5 rounded-full bg-green-500" />
					</span>
				) : (
					<span
						className={`size-2.5 rounded-full shrink-0 ${dotClass}`}
						aria-hidden="true"
					/>
				)}
				<MonitorIcon aria-hidden="true" className="size-4 text-muted-foreground" />
			</TooltipTrigger>
			<TooltipContent className="max-w-xs bg-zinc-800 text-zinc-100">
				<p className="text-xs font-medium">{label}</p>
				{errorMessage && (
					<p className="mt-1 text-xs text-zinc-300">{errorMessage}</p>
				)}
			</TooltipContent>
		</Tooltip>
	)
}

// ============================================================
// OAuthButton — only shown when auth is needed
// ============================================================

function OAuthButton({
	status,
	disabled,
	onClick,
}: {
	status: McpStatus
	disabled?: boolean
	onClick: () => void
}) {
	const needsAuth =
		status.status === "needs_auth" || status.status === "needs_client_registration"
	if (!needsAuth) return null

	const label =
		status.status === "needs_auth" ? "Authenticate" : "Register Client"

	return (
		<Tooltip>
			<TooltipTrigger
				render={
					<Button
						variant="ghost"
						size="icon"
						className="size-7"
						onClick={onClick}
						disabled={disabled}
					/>
				}
			>
				<KeyRoundIcon aria-hidden="true" className="size-3.5" />
			</TooltipTrigger>
			<TooltipContent side="top" className="bg-zinc-800 text-zinc-100">
				<p className="text-xs">{label}</p>
			</TooltipContent>
		</Tooltip>
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
	const { install, cancel, getState } = useMcpInstall()
	const setChatInitDir = useSetAtom(chatInitDirectoryAtom)
	const [startingServer, setStartingServer] = useState(false)
	const [togglingServers, setTogglingServers] = useState<Set<string>>(new Set())
	const [activeTab, setActiveTab] = useState("servers")
	const [installPage, setInstallPage] = useState(0)

	const handleStartServer = async () => {
		setStartingServer(true)
		try {
			await fetchOpenCodeUrl()
		} catch {
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

	const handleOAuth = async (server: McpServerInfo) => {
		try {
			if (server.status.status === "needs_auth") {
				await startAuth(server.name)
				toast.success(`OAuth flow started for ${server.name}`)
			} else if (server.status.status === "needs_client_registration") {
				await authenticate(server.name)
				toast.success(`Client registration completed for ${server.name}`)
			}
		} catch (err) {
			toast.error(
				err instanceof Error ? err.message : `OAuth operation failed for ${server.name}`,
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

	const servers = data ?? []
	const installedNames = servers.map((s) => s.name)
	const anyToggling = togglingServers.size > 0

	const perPage = 6
	const totalInstallPages = Math.ceil(MCP_TEMPLATES.length / perPage)
	const visibleTemplates = MCP_TEMPLATES.slice(
		installPage * perPage,
		(installPage + 1) * perPage,
	)

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
		<div className="space-y-6">
			<div className="flex items-start justify-between">
				<div>
					<h2 className="text-xl font-semibold">MCP Servers</h2>
					<p className="mt-1 text-sm text-muted-foreground">
						Manage MCP servers that extend agent capabilities with external tools and
						resources.
					</p>
				</div>
				<Button size="sm" onClick={handleAddServer}>
					<BlocksIcon aria-hidden="true" className="mr-1.5 size-4" />
					Add MCP Server
				</Button>
			</div>

			<Tabs value={activeTab} onValueChange={setActiveTab}>
				<TabsList className="mb-4">
					<TabsTrigger value="servers">Configured Servers</TabsTrigger>
					<TabsTrigger value="install">Quick Install</TabsTrigger>
				</TabsList>

				<TabsContent value="servers">
					<SettingsSection>
						{loading && servers.length === 0 ? (
							<div className="space-y-4 p-4">
								<Skeleton className="h-10 w-full" />
								<Skeleton className="h-10 w-full" />
								<Skeleton className="h-10 w-full" />
							</div>
						) : servers.length === 0 ? (
							<div className="flex flex-col items-center justify-center py-12 text-center">
								<BlocksIcon
									aria-hidden="true"
									className="mb-3 size-10 text-muted-foreground"
								/>
								<p className="text-sm text-muted-foreground">
									No MCP servers configured.
								</p>
								<p className="mt-1 text-xs text-muted-foreground">
									Add an MCP server to extend your agents with custom tools,
									resources, and prompts.
								</p>
								<Button
									variant="outline"
									size="sm"
									className="mt-4"
									onClick={handleAddServer}
								>
									<BlocksIcon aria-hidden="true" className="mr-1.5 size-4" />
									Add MCP Server
								</Button>
							</div>
						) : (
							<div className="divide-y">
								{servers.map((server) => {
									const isEnabled = server.status.status !== "disabled"
									return (
										<div
											key={server.name}
											className="py-3 first:pt-0 last:pb-0"
										>
											<SettingsRow
												label={server.name}
												description={server.subtitle}
												htmlFor={`mcp-toggle-${server.name}`}
											>
												<div className="flex items-center gap-2">
													<OAuthButton
														status={server.status}
														disabled={anyToggling}
														onClick={() => handleOAuth(server)}
													/>
													<StatusIndicator status={server.status} />
													<span
														className="mx-1 h-4 w-px shrink-0 bg-border"
														aria-hidden="true"
													/>
													<Switch
														id={`mcp-toggle-${server.name}`}
														checked={isEnabled}
														onCheckedChange={(enabled) =>
															handleToggle(server.name, enabled)
														}
														disabled={anyToggling}
													/>
												</div>
											</SettingsRow>
										</div>
									)
								})}
							</div>
						)}
					</SettingsSection>
				</TabsContent>

				<TabsContent value="install">
					<SettingsSection>
						<div className="grid grid-cols-2 gap-2 p-4">
							{visibleTemplates.map((template) => (
								<McpTemplateCard
									key={template.id}
									template={template}
									state={getState(template.id)}
									installed={isTemplateInstalled(template.id, installedNames)}
									onInstall={install}
									onCancel={() => cancel(template.id)}
								/>
							))}
						</div>
						{totalInstallPages > 1 && (
							<div className="flex items-center justify-center gap-1 pb-3">
								<Button
									size="sm"
									variant="ghost"
									className="h-7 w-7 p-0"
									onClick={() => setInstallPage((p) => p - 1)}
									disabled={installPage === 0}
								>
									<ChevronLeftIcon aria-hidden="true" className="size-4" />
								</Button>
								<span className="min-w-12 text-center text-xs text-muted-foreground tabular-nums">
									{installPage + 1} / {totalInstallPages}
								</span>
								<Button
									size="sm"
									variant="ghost"
									className="h-7 w-7 p-0"
									onClick={() => setInstallPage((p) => p + 1)}
									disabled={installPage === totalInstallPages - 1}
								>
									<ChevronRightIcon aria-hidden="true" className="size-4" />
								</Button>
							</div>
						)}
					</SettingsSection>
				</TabsContent>
			</Tabs>
		</div>
	)
}
