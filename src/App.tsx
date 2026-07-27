import { FolderOpen, MessageSquarePlus } from "lucide-react"
import { AppShell } from "@/components/layout/app-shell"

export default function App() {
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
							className="flex items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm text-muted transition hover:bg-white/5 hover:text-fg"
							disabled
							title="Coming in Phase 1"
						>
							<FolderOpen className="size-4 shrink-0" />
							Open project
						</button>
						<button
							type="button"
							className="flex items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm text-muted transition hover:bg-white/5 hover:text-fg"
							disabled
							title="Coming in Phase 1"
						>
							<MessageSquarePlus className="size-4 shrink-0" />
							New chat
						</button>
						<div className="mt-4 px-2.5 text-[11px] uppercase tracking-wider text-muted/70">
							Projects
						</div>
						<p className="px-2.5 text-xs text-muted">
							No projects yet. Scaffold is ready — ACP wiring is next.
						</p>
					</div>
					<div className="border-t border-border px-4 py-3 text-[11px] text-muted">
						ACP · OpenCode first
					</div>
				</>
			}
		>
			<div className="flex h-12 items-center border-b border-border px-4 text-xs text-muted">
				No active session
			</div>
			<div className="flex flex-1 flex-col items-center justify-center gap-3 px-8 text-center">
				<p className="text-lg font-medium tracking-tight">Circulo</p>
				<p className="max-w-md text-sm text-muted">
					Native ACP desktop client. Visual language inspired by Palot; transport is
					JSON-RPC stdio — not OpenCode HTTP/SSE.
				</p>
				<p className="max-w-md text-xs text-muted/80">
					See <code className="text-fg/80">docs/PRD.md</code> and{" "}
					<code className="text-fg/80">docs/TRD.md</code> for the MVP plan.
				</p>
			</div>
			<div className="border-t border-border px-4 py-3">
				<div className="mx-auto max-w-3xl rounded-lg border border-border bg-surface px-3 py-2 text-sm text-muted">
					Composer disabled until a session is ready
				</div>
			</div>
		</AppShell>
	)
}
