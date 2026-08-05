import { FolderOpen, Plus, Trash2 } from "lucide-react"
import { SectionHeader } from "@/components/settings/sections/section-ui"
import { projectName } from "@/lib/workspace"
import { cn } from "@/lib/utils"
import type { RecentProject, WorkspaceEntry } from "@/types/acp"

interface WorkspacesSectionProps {
	workspaces: WorkspaceEntry[]
	activeWorkspaceId: string | null
	recentProjects: RecentProject[]
	onAddWorkspace: () => void
	onDeleteWorkspace: (workspaceId: string) => void
	onSelectWorkspace: (workspaceId: string) => void
	onClose: () => void
}

export function WorkspacesSection({
	workspaces,
	activeWorkspaceId,
	recentProjects,
	onAddWorkspace,
	onDeleteWorkspace,
	onSelectWorkspace,
	onClose,
}: WorkspacesSectionProps) {
	const canDelete = workspaces.length > 1

	return (
		<div>
			<SectionHeader
				title="Workspaces"
				description="Each space has its own chats folder and project list."
			/>
			<div className="space-y-3">
				<div className="space-y-1.5">
					{workspaces.map((ws, index) => {
						const active = ws.id === activeWorkspaceId
						return (
							<div
								key={ws.id}
								className={cn(
									"flex items-center justify-between gap-2 rounded-lg border bg-black/20 px-3.5 py-2.5",
									active ? "border-white/25" : "border-border",
								)}
							>
								<button
									type="button"
									onClick={() => {
										if (!active) {
											onSelectWorkspace(ws.id)
											onClose()
										}
									}}
									disabled={active}
									className="flex min-w-0 flex-1 items-center gap-2.5 text-left disabled:cursor-default"
									title={active ? "Current space" : `Switch to space ${index + 1}`}
								>
									<span
										className={cn(
											"block size-1.5 shrink-0 rounded-full",
											active ? "bg-fg" : "bg-muted/50",
										)}
									/>
									<span className="min-w-0 flex-1">
										<span
											className={cn(
												"block truncate text-sm",
												active ? "font-medium text-fg" : "text-fg/85",
											)}
										>
											Space {index + 1}
											{active ? (
												<span className="ml-2 text-[10px] font-normal uppercase tracking-wider text-muted">
													active
												</span>
											) : null}
										</span>
										<span className="block truncate text-[11px] text-muted">
											{ws.projectPaths.length} project
											{ws.projectPaths.length === 1 ? "" : "s"}
										</span>
									</span>
								</button>
								<button
									type="button"
									disabled={!canDelete}
									onClick={() => onDeleteWorkspace(ws.id)}
									className="shrink-0 rounded p-1.5 text-muted transition hover:bg-white/5 hover:text-red-300 disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-muted"
									title={
										canDelete
											? "Delete workspace"
											: "Cannot delete the last workspace"
									}
								>
									<Trash2 className="size-3.5" />
								</button>
							</div>
						)
					})}
				</div>

				<button
					type="button"
					onClick={onAddWorkspace}
					className="flex items-center gap-2 rounded-lg border border-dashed border-border px-3.5 py-2 text-sm text-fg/85 transition hover:bg-white/[0.04]"
				>
					<Plus className="size-4" />
					Add workspace
				</button>

				{recentProjects.length > 0 ? (
					<div className="rounded-lg border border-border bg-black/20 p-3.5">
						<div className="mb-2 flex items-center gap-1.5 text-[11px] uppercase tracking-wider text-muted">
							<FolderOpen className="size-3" />
							Recent projects
						</div>
						<div className="space-y-1">
							{recentProjects.slice(0, 8).map((project) => (
								<div
									key={project.path}
									className="truncate px-1 py-0.5 font-mono text-[11px] text-fg/80"
									title={project.path}
								>
									{projectName(project.path)}
								</div>
							))}
						</div>
					</div>
				) : null}
			</div>
		</div>
	)
}
