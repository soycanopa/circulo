import { FolderTree, X } from "lucide-react"
import { FileTree } from "@/components/layout/file-tree"
import { projectName } from "@/lib/workspace"

interface FileTreePanelProps {
	projectPath: string | null
	onClose: () => void
	onOpenFile: (path: string) => void
	onMentionFile: (relativePath: string) => void
}

/** Right-side file browser panel, styled like the diff panel. */
export function FileTreePanel({
	projectPath,
	onClose,
	onOpenFile,
	onMentionFile,
}: FileTreePanelProps) {
	return (
		<aside className="flex h-full w-full flex-col overflow-hidden rounded-tr-[8px] rounded-br-[8px]">
			<div
				className="flex h-12 shrink-0 items-center justify-between border-b border-border px-4 pb-0.5"
				data-tauri-drag-region="deep"
			>
				<div className="flex min-w-0 items-center gap-2">
					<FolderTree className="size-4 shrink-0 text-violet-300" />
					<div className="min-w-0">
						<p className="truncate text-sm font-medium text-fg">Files</p>
						{projectPath ? (
							<p className="truncate text-xs text-muted">
								{projectName(projectPath)}
							</p>
						) : null}
					</div>
				</div>
				<button
					type="button"
					onClick={onClose}
					className="rounded p-1 text-muted transition hover:bg-white/5 hover:text-fg"
					title="Close file panel"
					data-tauri-drag-region="false"
				>
					<X className="size-4" />
				</button>
			</div>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				<FileTree
					rootPath={projectPath}
					onOpenFile={onOpenFile}
					onMentionFile={onMentionFile}
				/>
			</div>
		</aside>
	)
}
