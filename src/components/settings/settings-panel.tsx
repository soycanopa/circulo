import { X } from "lucide-react"
import { getDefaultChatsPath } from "@/lib/tauri"
import { useEffect, useState } from "react"

interface SettingsPanelProps {
	open: boolean
	onClose: () => void
	agentCommand: string
}

export function SettingsPanel({ open, onClose, agentCommand }: SettingsPanelProps) {
	const [chatsPath, setChatsPath] = useState("—")

	useEffect(() => {
		if (!open) return
		void getDefaultChatsPath().then(setChatsPath)
	}, [open])

	if (!open) return null

	return (
		<div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
			<div
				role="dialog"
				aria-modal="true"
				className="w-full max-w-md rounded-lg border border-border bg-sidebar shadow-xl"
			>
				<div className="flex items-center justify-between border-b border-border px-4 py-3">
					<h2 className="text-sm font-medium text-fg">Settings</h2>
					<button
						type="button"
						onClick={onClose}
						className="rounded p-1 text-muted transition hover:bg-white/5 hover:text-fg"
					>
						<X className="size-4" />
					</button>
				</div>
				<div className="space-y-4 px-4 py-4 text-xs">
					<div>
						<div className="text-[11px] uppercase tracking-wider text-muted">
							Agent
						</div>
						<p className="mt-1 font-mono text-fg">{agentCommand}</p>
					</div>
					<div>
						<div className="text-[11px] uppercase tracking-wider text-muted">
							General chats folder
						</div>
						<p className="mt-1 break-all font-mono text-fg/90">{chatsPath}</p>
					</div>
					<div>
						<div className="text-[11px] uppercase tracking-wider text-muted">
							About
						</div>
						<p className="mt-1 text-muted">
							Circulo v0.1.0 — desktop ACP client for OpenCode
						</p>
					</div>
				</div>
			</div>
		</div>
	)
}
