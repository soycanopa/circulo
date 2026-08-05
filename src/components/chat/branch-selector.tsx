import { useSetAtom } from "jotai"
import {
	Check,
	ChevronDown,
	GitBranch,
	Loader2,
	Plus,
	Search,
} from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import { Input } from "@/components/ui/input"
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "@/components/ui/popover"
import { useGitBranches } from "@/hooks/use-git-branches"
import { gitCheckout, gitCreateBranch } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import { errorMessageAtom, gitRefreshVersionAtom } from "@/stores/atoms"
import type { GitBranchInfo, GitBranches } from "@/types/acp"

interface BranchSelectorProps {
	projectPath: string | null
}

export function BranchSelector({ projectPath }: BranchSelectorProps) {
	const { branches, loading, isRepo } = useGitBranches(projectPath)
	const setError = useSetAtom(errorMessageAtom)
	const setGitRefreshVersion = useSetAtom(gitRefreshVersionAtom)

	const [open, setOpen] = useState(false)
	const [search, setSearch] = useState("")
	const [list, setList] = useState<GitBranches | null>(null)
	const [creating, setCreating] = useState(false)
	const [newName, setNewName] = useState("")
	const [creatingBusy, setCreatingBusy] = useState(false)
	const [busyName, setBusyName] = useState<string | null>(null)
	const [localError, setLocalError] = useState<string | null>(null)
	const inputRef = useRef<HTMLInputElement>(null)

	// Mirror the hook result so the popover shows the freshest list after a
	// checkout/create without waiting for the background refresh round-trip.
	useEffect(() => {
		setList(branches)
	}, [branches])

	const data = list ?? branches
	const busy = busyName !== null || creatingBusy

	const filteredLocal = useMemo(() => {
		if (!data) return []
		const q = search.trim().toLowerCase()
		return data.local.filter(
			(branch) => !q || branch.name.toLowerCase().includes(q),
		)
	}, [data, search])

	const filteredRemote = useMemo(() => {
		if (!data) return []
		const q = search.trim().toLowerCase()
		return data.remote.filter(
			(branch) => !q || branch.name.toLowerCase().includes(q),
		)
	}, [data, search])

	if (!projectPath || !isRepo) return null

	const currentLabel = data?.current ?? "…"
	const hasSelection = Boolean(data)

	function applyResult(result: GitBranches) {
		setList(result)
		setGitRefreshVersion((version) => version + 1)
	}

	async function handleSelect(branch: GitBranchInfo) {
		if (!projectPath || busy) return
		setBusyName(branch.name)
		setLocalError(null)
		try {
			const result = await gitCheckout(projectPath, branch.name)
			applyResult(result)
			setOpen(false)
		} catch (err) {
			const message =
				err instanceof Error ? err.message : "Checkout failed"
			setLocalError(message)
			setError(message)
		} finally {
			setBusyName(null)
		}
	}

	async function handleCreate() {
		const trimmed = newName.trim()
		if (!projectPath || !trimmed || creatingBusy) return
		setCreatingBusy(true)
		setLocalError(null)
		try {
			const result = await gitCreateBranch(
				projectPath,
				trimmed,
				data?.current || undefined,
			)
			applyResult(result)
			setCreating(false)
			setNewName("")
			setOpen(false)
		} catch (err) {
			const message =
				err instanceof Error ? err.message : "Create branch failed"
			setLocalError(message)
			setError(message)
		} finally {
			setCreatingBusy(false)
		}
	}

	function renderRow(branch: GitBranchInfo) {
		return (
			<button
				key={branch.name}
				type="button"
				disabled={busy}
				onClick={() => void handleSelect(branch)}
				className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-xs text-white/80 transition-colors hover:bg-white/[0.08] hover:text-white/90 disabled:cursor-not-allowed disabled:opacity-50"
			>
				{branch.current ? (
					<Check className="size-3.5 shrink-0 text-green-400" />
				) : (
					<span className="size-3.5 shrink-0" />
				)}
				<span className="min-w-0 flex-1 truncate">{branch.name}</span>
				{branch.upstream ? (
					<span className="shrink-0 text-[10px] text-white/35">
						{branch.upstream}
					</span>
				) : null}
				{busyName === branch.name ? (
					<Loader2 className="size-3.5 shrink-0 animate-spin text-white/50" />
				) : null}
			</button>
		)
	}

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<button
					type="button"
					aria-label="Git branch"
					title={`Current branch: ${currentLabel}`}
					className="group inline-flex max-w-full items-center gap-1.5 rounded-md px-2 py-0.5 text-[11px] text-white/55 shadow-none transition-colors hover:bg-white/[0.08] hover:text-white/85 focus-visible:bg-white/[0.08] focus-visible:outline-none data-[state=open]:bg-white/[0.08]"
				>
					<GitBranch
						className={cn(
							"size-3 shrink-0 transition-colors group-hover:text-white/70",
							data?.detached ? "text-amber-400/80" : "text-white/45",
						)}
					/>
					<span
						className={cn(
							"max-w-44 truncate",
							data?.detached && "text-amber-400/90",
						)}
					>
						{currentLabel}
					</span>
					{loading ? (
						<Loader2 className="size-2.5 shrink-0 animate-spin text-white/40" />
					) : (
						<ChevronDown className="size-3 shrink-0 text-white/40 transition-colors group-hover:text-white/60" />
					)}
				</button>
			</PopoverTrigger>
			<PopoverContent align="start" side="top" className="w-80 p-0">
				<div className="border-b border-border p-2">
					<div className="relative">
						<Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-white/40" />
						<Input
							ref={inputRef}
							value={search}
							onChange={(event) => setSearch(event.target.value)}
							placeholder="Filter branches…"
							className="pl-8"
						/>
					</div>
				</div>

				<div className="max-h-72 overflow-y-auto p-1.5">
					{!hasSelection ? (
						<div className="px-2 py-4 text-center text-xs text-white/40">
							Loading branches…
						</div>
					) : (
						<>
							{localError ? (
								<div className="mx-1 mb-1.5 rounded border border-red-500/30 bg-red-500/10 px-2 py-1.5 text-[11px] text-red-200">
									{localError}
								</div>
							) : null}

							{filteredLocal.length > 0 ? (
								<div className="mb-1 px-2 pt-1 text-[10px] font-medium uppercase tracking-wider text-white/35">
									Local
								</div>
							) : null}
							{filteredLocal.map(renderRow)}

							{filteredRemote.length > 0 ? (
								<div className="mb-1 mt-2 px-2 pt-1 text-[10px] font-medium uppercase tracking-wider text-white/35">
									Remote
								</div>
							) : null}
							{filteredRemote.map(renderRow)}

							{filteredLocal.length === 0 &&
							filteredRemote.length === 0 ? (
								<div className="px-2 py-4 text-center text-xs text-white/40">
									No branches match “{search}”.
								</div>
							) : null}
						</>
					)}
				</div>

				<div className="border-t border-border p-1.5">
					{creating ? (
						<form
							className="flex items-center gap-1.5 px-1 py-0.5"
							onSubmit={(event) => {
								event.preventDefault()
								void handleCreate()
							}}
						>
							<Input
								autoFocus
								value={newName}
								onChange={(event) => setNewName(event.target.value)}
								placeholder="New branch name"
								className="h-7 flex-1"
								disabled={creatingBusy}
							/>
							<button
								type="submit"
								disabled={creatingBusy || !newName.trim()}
								className="inline-flex h-7 shrink-0 items-center gap-1 rounded-md border border-white/15 bg-white/[0.06] px-2.5 text-xs text-white/85 transition-colors hover:bg-white/[0.12] disabled:cursor-not-allowed disabled:opacity-50"
							>
								{creatingBusy ? (
									<Loader2 className="size-3 animate-spin" />
								) : (
									<Check className="size-3" />
								)}
								Create &amp; switch
							</button>
						</form>
					) : (
						<button
							type="button"
							disabled={busy}
							onClick={() => {
								setCreating(true)
								setLocalError(null)
							}}
							className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-xs text-white/75 transition-colors hover:bg-white/[0.08] hover:text-white/90 disabled:cursor-not-allowed disabled:opacity-50"
						>
							<Plus className="size-3.5 text-white/55" />
							New branch
						</button>
					)}
				</div>
			</PopoverContent>
		</Popover>
	)
}
