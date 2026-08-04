import { useAtom, useAtomValue } from "jotai"
import { Terminal, X } from "lucide-react"
import { useEffect, useRef } from "react"
import { cn } from "@/lib/utils"
import {
	activeTerminalIdAtom,
	terminalsAtom,
} from "@/stores/atoms"

interface TerminalDrawerProps {
	onClose: () => void
}

export function TerminalDrawer({ onClose }: TerminalDrawerProps) {
	const [activeId, setActiveId] = useAtom(activeTerminalIdAtom)
	const terminals = useAtomValue(terminalsAtom)
	const scrollRef = useRef<HTMLPreElement>(null)

	const entries = Object.values(terminals)
	const active = activeId ? terminals[activeId] : entries[entries.length - 1]

	useEffect(() => {
		if (!activeId && entries.length > 0) {
			setActiveId(entries[entries.length - 1]!.terminalId)
		}
	}, [activeId, entries, setActiveId])

	useEffect(() => {
		scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight })
	}, [active?.output, active?.running])

	return (
		<div className="flex h-full min-h-0 flex-col border-t border-border bg-black/40">
			<div className="flex h-10 shrink-0 items-center justify-between gap-2 border-b border-border px-3">
				<div className="flex min-w-0 items-center gap-2">
					<Terminal className="size-3.5 shrink-0 text-emerald-300" />
					<p className="truncate text-xs font-medium text-fg">
						{active?.label ?? "Terminal"}
					</p>
					{active ? (
						<span
							className={cn(
								"shrink-0 rounded px-1.5 py-0.5 text-[10px] uppercase tracking-wide",
								active.running
									? "bg-amber-500/15 text-amber-200"
									: "bg-emerald-500/15 text-emerald-300",
							)}
						>
							{active.running ? "running" : "exited"}
						</span>
					) : null}
				</div>
				<button
					type="button"
					onClick={onClose}
					className="rounded p-1 text-muted transition hover:bg-white/5 hover:text-fg"
					title="Close terminal"
				>
					<X className="size-4" />
				</button>
			</div>

			{entries.length > 1 ? (
				<div className="flex shrink-0 gap-1 overflow-x-auto border-b border-border px-2 py-1">
					{entries.map((entry) => (
						<button
							key={entry.terminalId}
							type="button"
							onClick={() => setActiveId(entry.terminalId)}
							className={cn(
								"shrink-0 rounded px-2 py-0.5 text-[10px] transition",
								entry.terminalId === active?.terminalId
									? "bg-white/10 text-fg"
									: "text-muted hover:bg-white/5 hover:text-fg",
							)}
						>
							{entry.label}
						</button>
					))}
				</div>
			) : null}

			<pre
				ref={scrollRef}
				className="min-h-0 flex-1 overflow-auto px-3 py-2 font-mono text-[11px] leading-relaxed text-emerald-100"
			>
				{active?.output || "Waiting for terminal output…"}
				{active?.truncated ? (
					<span className="block pt-2 text-[10px] text-muted">
						(output truncated)
					</span>
				) : null}
				{active?.exitStatus && !active.running ? (
					<span className="block pt-2 text-[10px] text-muted">
						exit{" "}
						{active.exitStatus.exitCode !== undefined
							? active.exitStatus.exitCode
							: active.exitStatus.signal ?? "unknown"}
					</span>
				) : null}
			</pre>
		</div>
	)
}
