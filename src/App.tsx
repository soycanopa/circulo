import { useAtom, useAtomValue, useSetAtom } from "jotai"
import { FolderOpen, Loader2, MessageSquarePlus } from "lucide-react"
import { useState } from "react"
import { ChatInput } from "@/components/chat/chat-input"
import { ConfigSelectors } from "@/components/chat/config-selector"
import { MessageList } from "@/components/chat/message-list"
import { AppShell } from "@/components/layout/app-shell"
import { useAcpBridge } from "@/hooks/use-acp-bridge"
import { useBootstrapAgent } from "@/hooks/use-bootstrap"
import {
	createSession,
	getDefaultChatsPath,
	openProject,
	pickDirectory,
} from "@/lib/tauri"
import {
	agentConnectedAtom,
	errorMessageAtom,
	messagesAtom,
	projectPathAtom,
	sessionIdAtom,
	sessionStatusAtom,
	streamingTextAtom,
} from "@/stores/atoms"

function isChatsWorkspace(path: string | null): boolean {
	return Boolean(path?.includes("/.circulo/chats"))
}

export default function App() {
	useAcpBridge()
	useBootstrapAgent()

	const projectPath = useAtomValue(projectPathAtom)
	const sessionId = useAtomValue(sessionIdAtom)
	const connected = useAtomValue(agentConnectedAtom)
	const status = useAtomValue(sessionStatusAtom)
	const error = useAtomValue(errorMessageAtom)
	const setError = useSetAtom(errorMessageAtom)
	const setStatus = useSetAtom(sessionStatusAtom)
	const setMessages = useSetAtom(messagesAtom)
	const setStreaming = useSetAtom(streamingTextAtom)
	const [, setSessionId] = useAtom(sessionIdAtom)

	const [busy, setBusy] = useState(false)
	const warming = status === "connecting" && !sessionId
	const ready = Boolean(sessionId && connected)

	async function ensureAgentWarm(): Promise<void> {
		if (connected && sessionId && projectPath) return
		const chatsPath = await getDefaultChatsPath()
		await openProject(chatsPath)
	}

	async function handleOpenProject() {
		setError(null)
		const path = await pickDirectory()
		if (!path) return

		const base = path.split("/").filter(Boolean).pop() ?? path
		const largeRoot =
			base === "Desktop" ||
			base === "Documents" ||
			base === "Downloads" ||
			path === "/Users/soycanopa"

		setBusy(true)
		setStatus("connecting")
		setMessages([])
		setStreaming("")
		setSessionId(null)
		try {
			await openProject(path)
			if (largeRoot) {
				setError(
					"Sesión lista. Carpetas grandes (Desktop/Home) hacen el arranque lento — prefiera un repo concreto.",
				)
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to open project")
			setStatus("idle")
		} finally {
			setBusy(false)
		}
	}

	async function handleNewChat() {
		setError(null)
		setBusy(true)
		setStatus("connecting")
		setMessages([])
		setStreaming("")
		try {
			// Always allowed: warm agent on chats workspace if needed, then session/new.
			if (!connected || !projectPath) {
				await ensureAgentWarm()
			} else {
				await createSession()
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to create session")
			setStatus("idle")
		} finally {
			setBusy(false)
		}
	}

	const workspaceLabel = isChatsWorkspace(projectPath)
		? "General Chat"
		: projectPath
			? projectPath.split("/").filter(Boolean).slice(-2).join("/")
			: "Starting…"

	const statusLabel = warming
		? "Warming OpenCode (first launch can take a bit)…"
		: status === "connecting"
			? "Creating session…"
			: status === "generating"
				? "Agent working…"
				: status === "awaiting_permission"
					? "Waiting for permission…"
					: ready
						? "Ready"
						: "Idle"

	return (
		<AppShell
			sidebar={
				<>
					<div className="flex h-12 items-center border-b border-border px-4 text-sm font-medium tracking-tight">
						Circulo
					</div>
					<div className="flex flex-1 flex-col gap-1 p-3">
						<button
							type="button"
							onClick={() => void handleNewChat()}
							disabled={busy || warming}
							className="flex items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm text-fg/90 transition hover:bg-white/5 disabled:opacity-40"
						>
							{busy || warming ? (
								<Loader2 className="size-4 shrink-0 animate-spin" />
							) : (
								<MessageSquarePlus className="size-4 shrink-0" />
							)}
							New chat
						</button>
						<button
							type="button"
							onClick={() => void handleOpenProject()}
							disabled={busy || warming}
							className="flex items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm text-fg/90 transition hover:bg-white/5 disabled:opacity-50"
						>
							<FolderOpen className="size-4 shrink-0" />
							Open project
						</button>
						<div className="mt-4 px-2.5 text-[11px] uppercase tracking-wider text-muted/70">
							Workspace
						</div>
						<p className="truncate px-2.5 text-xs text-muted">{workspaceLabel}</p>
						{sessionId ? (
							<p className="mt-1 truncate px-2.5 font-mono text-[10px] text-muted/70">
								{sessionId}
							</p>
						) : null}
					</div>
					<div className="border-t border-border px-4 py-3 text-[11px] text-muted">
						{ready ? "Agent warm" : warming ? "Warming…" : "Agent idle"} · ACP
					</div>
				</>
			}
		>
			<div className="flex h-12 items-center justify-between gap-3 border-b border-border px-4">
				<div className="min-w-0 text-xs text-muted">{statusLabel}</div>
				<ConfigSelectors />
			</div>

			{error ? (
				<div className="mx-4 mt-3 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200">
					{error}
				</div>
			) : null}

			{warming && !sessionId ? (
				<div className="flex flex-1 flex-col items-center justify-center gap-3 px-8 text-center">
					<Loader2 className="size-6 animate-spin text-muted" />
					<p className="text-sm font-medium">Starting agent…</p>
					<p className="max-w-md text-xs text-muted">
						OpenCode se calienta al abrir la app en{" "}
						<code className="text-fg/80">~/.circulo/chats</code>. Después New Chat y
						el composer quedan listos sin abrir un proyecto.
					</p>
				</div>
			) : (
				<MessageList />
			)}

			<ChatInput />
		</AppShell>
	)
}
