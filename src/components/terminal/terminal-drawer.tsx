import { useAtom, useAtomValue } from "jotai"
import { Plus, Terminal, X } from "lucide-react"
import { useEffect, useRef } from "react"
import { cn } from "@/lib/utils"
import { EmbeddedTerminal, cancelEmbeddedTerminalClose } from "@/components/terminal/embedded-terminal"
import { ensureTerminalOutputListener } from "@/lib/terminal-output-bridge"
import { closeUserTerminal, spawnUserTerminal } from "@/lib/tauri"
import {
	activeTerminalIdAtom,
	terminalsAtom,
	userTerminalTabsAtom,
	type UserTerminalTab,
} from "@/stores/atoms"

export const AGENT_TERMINAL_PREFIX = "agent:"

export function agentTerminalTabId(terminalId: string): string {
	return `${AGENT_TERMINAL_PREFIX}${terminalId}`
}

export function isAgentTerminalTab(tabId: string): boolean {
	return tabId.startsWith(AGENT_TERMINAL_PREFIX)
}

function createShellTab(existing: UserTerminalTab[]): UserTerminalTab {
	return {
		id: crypto.randomUUID(),
		title: `Terminal ${existing.length + 1}`,
	}
}

interface TerminalDrawerProps {
	projectPath: string | null
	onClose: () => void
}

export function TerminalDrawer({ projectPath, onClose }: TerminalDrawerProps) {
	const [activeId, setActiveId] = useAtom(activeTerminalIdAtom)
	const [shellTabs, setShellTabs] = useAtom(userTerminalTabsAtom)
	const agentTerminals = useAtomValue(terminalsAtom)
	const scrollRef = useRef<HTMLPreElement>(null)

	const agentEntries = Object.values(agentTerminals)
	const activeAgentId =
		activeId && isAgentTerminalTab(activeId)
			? activeId.slice(AGENT_TERMINAL_PREFIX.length)
			: null
	const agentActive = activeAgentId ? agentTerminals[activeAgentId] : undefined
	const activeShellId =
		activeId && !isAgentTerminalTab(activeId) ? activeId : null

	useEffect(() => {
		if (shellTabs.length > 0) return
		const first = createShellTab([])
		setShellTabs([first])
		setActiveId(first.id)
	}, [setActiveId, setShellTabs, shellTabs.length])

	useEffect(() => {
		if (!projectPath || shellTabs.length === 0) return
		const tab = shellTabs[0]
		if (!tab) return
		void (async () => {
			await ensureTerminalOutputListener()
			await spawnUserTerminal(tab.id, projectPath, 80, 24).catch(() => {})
		})()
	}, [projectPath, shellTabs])

	useEffect(() => {
		scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight })
	}, [agentActive?.output, agentActive?.running])

	function addShellTab() {
		const tab = createShellTab(shellTabs)
		setShellTabs((prev) => [...prev, tab])
		setActiveId(tab.id)
		if (projectPath) {
			void (async () => {
				await ensureTerminalOutputListener()
				await spawnUserTerminal(tab.id, projectPath, 80, 24).catch(() => {})
			})()
		}
	}

	function closeShellTab(tabId: string) {
		cancelEmbeddedTerminalClose(tabId)
		void closeUserTerminal(tabId)
		setShellTabs((prev) => {
			const next = prev.filter((tab) => tab.id !== tabId)
			if (activeId === tabId) {
				const fallback =
					next[next.length - 1]?.id ??
					(agentEntries[0]
						? agentTerminalTabId(agentEntries[0]!.terminalId)
						: null)
				setActiveId(fallback)
			}
			if (next.length === 0) {
				const fresh = createShellTab([])
				setActiveId(fresh.id)
				return [fresh]
			}
			return next
		})
	}

	return (
		<div className="flex h-full min-h-0 flex-col bg-content">
			<div className="flex h-9 shrink-0 items-center gap-1 overflow-x-auto border-b border-border px-2">
				{shellTabs.map((tab) => (
					<div
						key={tab.id}
						className={cn(
							"group flex shrink-0 items-center gap-1 rounded px-2 py-1 text-[10px] transition",
							activeShellId === tab.id
								? "bg-white/10 text-fg"
								: "text-muted hover:bg-white/5 hover:text-fg",
						)}
					>
						<button
							type="button"
							onClick={() => setActiveId(tab.id)}
							className="max-w-[8rem] truncate"
							title={tab.title}
						>
							{tab.title}
						</button>
						<button
							type="button"
							onClick={() => closeShellTab(tab.id)}
							className="rounded p-0.5 opacity-60 transition hover:bg-white/10 hover:opacity-100"
							title="Close tab"
							aria-label={`Close ${tab.title}`}
						>
							<X className="size-2.5" />
						</button>
					</div>
				))}
				{agentEntries.map((entry) => {
					const tabId = agentTerminalTabId(entry.terminalId)
					return (
						<button
							key={tabId}
							type="button"
							onClick={() => setActiveId(tabId)}
							className={cn(
								"max-w-[10rem] shrink-0 truncate rounded px-2 py-1 text-[10px] transition",
								activeId === tabId
									? "bg-white/10 text-fg"
									: "text-muted hover:bg-white/5 hover:text-fg",
							)}
							title={entry.label}
						>
							{entry.label}
						</button>
					)
				})}
				<button
					type="button"
					onClick={addShellTab}
					className="inline-flex shrink-0 items-center justify-center rounded p-1 text-muted transition hover:bg-white/5 hover:text-fg"
					title="New terminal tab"
					aria-label="New terminal tab"
				>
					<Plus className="size-3.5" />
				</button>
				<div className="ml-auto flex shrink-0 items-center gap-2 pl-2">
					<Terminal className="size-3 text-muted" />
					<button
						type="button"
						onClick={onClose}
						className="rounded p-1 text-muted transition hover:bg-white/5 hover:text-fg"
						title="Close terminal panel"
						aria-label="Close terminal panel"
					>
						<X className="size-3.5" />
					</button>
				</div>
			</div>

			<div className="relative min-h-0 flex-1">
				{shellTabs.map((tab) => (
					<div
						key={tab.id}
						className={cn(
							"absolute inset-0",
							activeShellId === tab.id ? "block" : "hidden",
						)}
					>
						<EmbeddedTerminal
							tabId={tab.id}
							projectPath={projectPath}
							isActive={activeShellId === tab.id}
						/>
					</div>
				))}
				{activeAgentId ? (
					<pre
						ref={scrollRef}
						className="absolute inset-0 overflow-auto bg-content px-3 py-2 font-mono text-[11px] leading-relaxed text-fg"
					>
						{agentActive?.output || "Waiting for agent terminal output…"}
						{agentActive?.truncated ? (
							<span className="block pt-2 text-[10px] text-muted">
								(output truncated)
							</span>
						) : null}
						{agentActive?.exitStatus && !agentActive?.running ? (
							<span className="block pt-2 text-[10px] text-muted">
								exit{" "}
								{agentActive.exitStatus.exitCode !== undefined
									? agentActive.exitStatus.exitCode
									: agentActive.exitStatus.signal ?? "unknown"}
							</span>
						) : null}
					</pre>
				) : null}
			</div>
		</div>
	)
}
