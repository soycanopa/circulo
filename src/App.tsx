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
	progressMessageAtom,
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
	const progress = useAtomValue(progressMessageAtom)
	const setError = useSetAtom(errorMessageAtom)
	const setStatus = useSetAtom(sessionStatusAtom)
	const setMessages = useSetAtom(messagesAtom)
	const setStreaming = useSetAtom(streamingTextAtom)
	const [, setSessionId] = useAtom(sessionIdAtom)

	const [busy, setBusy] = useState(false)
	const agentWarm = connected
	const hasSession = Boolean(sessionId)

	async function ensureAgentWarm(): Promise<void> {
		if (connected && projectPath) return
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
			// Agent only — no session until New Chat.
			await openProject(path)
			if (largeRoot) {
				setError(
					"Agente listo. Carpetas grandes (Desktop/Home) hacen el arranque lento — prefiera un repo concreto.",
				)
			}
			setStatus("idle")
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
			if (!connected || !projectPath) {
				await ensureAgentWarm()
			}
			// Explicit session/new — never auto-created on agent start.
			await createSession()
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
			: "—"

	const statusLabel =
		status === "generating"
			? "Agent working…"
			: status === "awaiting_permission"
				? "Waiting for permission…"
				: status === "connecting"
					? progress || "Working…"
					: hasSession
						? "Session ready"
						: agentWarm
							? "Agent ready — New Chat to begin"
							: progress || "Agent warming in background…"

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
							disabled={busy}
							className="flex items-center gap-2 rounded-md bg-white/10 px-2.5 py-2 text-left text-sm font-medium text-fg transition hover:bg-white/15 disabled:opacity-40"
						>
							{busy && status === "connecting" ? (
								<Loader2 className="size-4 shrink-0 animate-spin" />
							) : (
								<MessageSquarePlus className="size-4 shrink-0" />
							)}
							New chat
						</button>
						<button
							type="button"
							onClick={() => void handleOpenProject()}
							disabled={busy}
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
						) : (
							<p className="mt-1 px-2.5 text-[10px] text-muted/70">No session</p>
						)}
					</div>
					<div className="border-t border-border px-4 py-3 text-[11px] text-muted">
						{agentWarm ? "Agent warm" : "Agent starting…"} · ACP
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

			{!hasSession ? (
				<div className="flex flex-1 flex-col items-center justify-center gap-3 px-8 text-center">
					<p className="text-lg font-medium tracking-tight">Circulo</p>
					<p className="max-w-md text-sm text-muted">
						{agentWarm
							? "Agent is ready. Start a conversation with New Chat — no session is created until then."
							: "Agent is warming in the background. You can still click New Chat; it will wait until the agent is up."}
					</p>
					<button
						type="button"
						onClick={() => void handleNewChat()}
						disabled={busy}
						className="mt-2 inline-flex items-center gap-2 rounded-md bg-white/10 px-3 py-1.5 text-sm text-fg transition hover:bg-white/15 disabled:opacity-40"
					>
						{busy ? (
							<Loader2 className="size-4 animate-spin" />
						) : (
							<MessageSquarePlus className="size-4" />
						)}
						New chat
					</button>
				</div>
			) : (
				<MessageList />
			)}

			<ChatInput />
		</AppShell>
	)
}
