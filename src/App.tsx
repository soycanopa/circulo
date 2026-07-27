import { useAtom, useAtomValue, useSetAtom } from "jotai"
import { FolderOpen, Loader2, MessageSquarePlus } from "lucide-react"
import { useState } from "react"
import { ChatInput } from "@/components/chat/chat-input"
import { ConfigSelectors } from "@/components/chat/config-selector"
import { MessageList } from "@/components/chat/message-list"
import { AppShell } from "@/components/layout/app-shell"
import { useAcpBridge } from "@/hooks/use-acp-bridge"
import {
	createSession,
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

export default function App() {
	useAcpBridge()

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

	async function handleOpenProject() {
		setError(null)
		const path = await pickDirectory()
		if (!path) return
		setBusy(true)
		setStatus("connecting")
		setMessages([])
		setStreaming("")
		setSessionId(null)
		try {
			await openProject(path)
			// session:ready event binds session id
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to open project")
			setStatus("idle")
		} finally {
			setBusy(false)
		}
	}

	async function handleNewChat() {
		if (!connected || !projectPath) return
		setError(null)
		setBusy(true)
		setStatus("connecting")
		setMessages([])
		setStreaming("")
		// Keep previous sessionId until new session is ready (bridge replaces it).
		try {
			await createSession()
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to create session")
			setStatus("idle")
		} finally {
			setBusy(false)
		}
	}

	const shortPath = projectPath
		? projectPath.split("/").filter(Boolean).slice(-2).join("/")
		: null

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
							onClick={() => void handleOpenProject()}
							disabled={busy}
							className="flex items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm text-fg/90 transition hover:bg-white/5 disabled:opacity-50"
						>
							{busy && status === "connecting" ? (
								<Loader2 className="size-4 shrink-0 animate-spin" />
							) : (
								<FolderOpen className="size-4 shrink-0" />
							)}
							Open project
						</button>
						<button
							type="button"
							onClick={() => void handleNewChat()}
							disabled={busy || !connected}
							className="flex items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm text-fg/90 transition hover:bg-white/5 disabled:opacity-40"
						>
							<MessageSquarePlus className="size-4 shrink-0" />
							New chat
						</button>
						<div className="mt-4 px-2.5 text-[11px] uppercase tracking-wider text-muted/70">
							Project
						</div>
						<p className="truncate px-2.5 text-xs text-muted">
							{shortPath ?? "None open"}
						</p>
						{sessionId ? (
							<p className="mt-1 truncate px-2.5 font-mono text-[10px] text-muted/70">
								{sessionId}
							</p>
						) : null}
					</div>
					<div className="border-t border-border px-4 py-3 text-[11px] text-muted">
						{connected ? "Agent connected" : "Agent idle"} · ACP
					</div>
				</>
			}
		>
			<div className="flex h-12 items-center justify-between gap-3 border-b border-border px-4">
				<div className="min-w-0 text-xs text-muted">
					{status === "connecting"
						? "Connecting / creating session…"
						: sessionId
							? "Session ready"
							: "No active session"}
				</div>
				<ConfigSelectors />
			</div>

			{error ? (
				<div className="mx-4 mt-3 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200">
					{error}
				</div>
			) : null}

			{!projectPath && !error ? (
				<div className="flex flex-1 flex-col items-center justify-center gap-3 px-8 text-center">
					<p className="text-lg font-medium tracking-tight">Circulo</p>
					<p className="max-w-md text-sm text-muted">
						Open a project to spawn OpenCode over ACP and start chatting.
					</p>
					<button
						type="button"
						onClick={() => void handleOpenProject()}
						className="mt-2 rounded-md bg-white/10 px-3 py-1.5 text-sm text-fg transition hover:bg-white/15"
					>
						Open project
					</button>
				</div>
			) : (
				<MessageList />
			)}

			<ChatInput />
		</AppShell>
	)
}
