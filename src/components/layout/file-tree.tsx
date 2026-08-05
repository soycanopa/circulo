import {
	AtSign,
	ChevronDown,
	ChevronRight,
	File,
	FolderOpen,
	Folder,
} from "lucide-react"
import { useState } from "react"
import { Spinner } from "@/components/ui/spinner"
import { projectName } from "@/lib/workspace"
import { readDirectory, type DirectoryEntry } from "@/lib/tauri"

interface FileTreeProps {
	rootPath: string | null
	onOpenFile: (path: string) => void
	/** Inserts `@relative/path` into the composer. */
	onMentionFile: (relativePath: string) => void
}

export function FileTree({ rootPath, onOpenFile, onMentionFile }: FileTreeProps) {
	if (!rootPath) {
		return (
			<p className="px-2.5 py-2 text-xs text-muted/80">
				Open a project to browse its files.
			</p>
		)
	}
	return (
		<DirNode
			path={rootPath}
			name={projectName(rootPath)}
			rootPath={rootPath}
			defaultOpen
			onOpenFile={onOpenFile}
			onMentionFile={onMentionFile}
		/>
	)
}

function DirNode({
	path,
	name,
	rootPath,
	defaultOpen = false,
	onOpenFile,
	onMentionFile,
}: {
	path: string
	name: string
	rootPath: string
	defaultOpen?: boolean
	onOpenFile: (path: string) => void
	onMentionFile: (relativePath: string) => void
}) {
	const [entries, setEntries] = useState<DirectoryEntry[] | null>(null)
	const [open, setOpen] = useState(defaultOpen)
	const [loading, setLoading] = useState(false)
	const [error, setError] = useState<string | null>(null)

	async function toggle() {
		if (open) {
			setOpen(false)
			return
		}
		if (entries === null) {
			setLoading(true)
			try {
				const result = await readDirectory(path)
				setEntries(result)
			} catch (err) {
				setError(
					err instanceof Error ? err.message : "Failed to read directory",
				)
			} finally {
				setLoading(false)
			}
		}
		setOpen(true)
	}

	return (
		<div>
			<button
				type="button"
				onClick={() => void toggle()}
				className="flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-left text-xs text-fg/85 transition hover:bg-white/[0.06]"
			>
				{open || loading ? (
					<ChevronDown className="size-3.5 shrink-0 text-muted" />
				) : (
					<ChevronRight className="size-3.5 shrink-0 text-muted" />
				)}
				{open ? (
					<FolderOpen className="size-3.5 shrink-0 text-sky-300/80" />
				) : (
					<Folder className="size-3.5 shrink-0 text-sky-300/60" />
				)}
				<span className="min-w-0 flex-1 truncate">{name}</span>
				{loading ? <Spinner className="size-3 shrink-0 text-muted" /> : null}
			</button>
			{open ? (
				<div className="ml-3 border-l border-border pl-1">
					{error ? (
						<p className="px-2 py-1 text-[11px] text-red-300">{error}</p>
					) : null}
					{entries?.map((entry) =>
						entry.isDir ? (
							<DirNode
								key={entry.path}
								path={entry.path}
								name={entry.name}
								rootPath={rootPath}
								onOpenFile={onOpenFile}
								onMentionFile={onMentionFile}
							/>
						) : (
							<FileRow
								key={entry.path}
								path={entry.path}
								name={entry.name}
								rootPath={rootPath}
								onOpenFile={onOpenFile}
								onMentionFile={onMentionFile}
							/>
						),
					)}
				</div>
			) : null}
		</div>
	)
}

function FileRow({
	path,
	name,
	rootPath,
	onOpenFile,
	onMentionFile,
}: {
	path: string
	name: string
	rootPath: string
	onOpenFile: (path: string) => void
	onMentionFile: (relativePath: string) => void
}) {
	const relative = relativePath(rootPath, path)
	return (
		<div className="group flex items-center gap-0.5 rounded-md pr-1 transition-colors hover:bg-white/[0.06]">
			<button
				type="button"
				onClick={() => onOpenFile(path)}
				title={path}
				className="flex min-w-0 flex-1 items-center gap-1.5 rounded-md px-2 py-1 text-left text-xs text-fg/75 transition hover:text-fg"
			>
				<File className="size-3.5 shrink-0 text-muted" />
				<span className="min-w-0 flex-1 truncate">{name}</span>
			</button>
			<button
				type="button"
				onClick={() => onMentionFile(relative)}
				title={`Insert @${relative}`}
				aria-label={`Mention ${name}`}
				className="rounded p-1 text-muted opacity-0 transition hover:bg-white/10 hover:text-fg group-hover:opacity-100"
			>
				<AtSign className="size-3" />
			</button>
		</div>
	)
}

function relativePath(rootPath: string, absolute: string): string {
	if (absolute.startsWith(`${rootPath}/`)) {
		return absolute.slice(rootPath.length + 1)
	}
	return absolute
}
