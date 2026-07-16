import { parseDiffFromFile } from "@pierre/diffs"
import { FileDiff, PatchDiff } from "@pierre/diffs/react"
import { useMemo, type ReactNode } from "react"
import { ensureUnifiedDiffFormat } from "@/lib/diff-normalize"
import { PIERRE_DIFF_OPTIONS } from "@/lib/pierre-diff-options"
import { cn } from "@/lib/utils"

function diffCacheKey(path: string, label: string, contents: string): string {
	return `${path}:${label}:${contents.length}:${contents.slice(0, 48)}`
}

interface PierreDiffFrameProps {
	title?: string
	className?: string
	expanded?: boolean
	fill?: boolean
	children: ReactNode
}

function PierreDiffFrame({
	title,
	className,
	expanded = false,
	fill = false,
	children,
}: PierreDiffFrameProps) {
	return (
		<div
			className={cn(
				"pierre-diff overflow-hidden rounded-md border border-border bg-[#141414]",
				fill && "flex h-full min-h-0 flex-col",
				className,
			)}
		>
			{title ? (
				<div className="shrink-0 border-b border-border/60 bg-muted/30 px-3 py-1.5 font-mono text-xs text-muted-foreground">
					{title}
				</div>
			) : null}
			<div
				className={cn(
					"scrollbar-thin overflow-auto",
					fill ? "min-h-0 flex-1" : expanded ? "min-h-[50vh]" : "max-h-64",
				)}
			>
				{children}
			</div>
		</div>
	)
}

interface PierreFileDiffProps {
	path: string
	oldText?: string
	newText: string
	className?: string
	expanded?: boolean
	fill?: boolean
}

export function PierreFileDiff({
	path,
	oldText,
	newText,
	className,
	expanded = false,
	fill = false,
}: PierreFileDiffProps) {
	const fileDiff = useMemo(() => {
		try {
			return parseDiffFromFile(
				{
					name: path,
					contents: oldText ?? "",
					cacheKey: diffCacheKey(path, "old", oldText ?? ""),
				},
				{
					name: path,
					contents: newText,
					cacheKey: diffCacheKey(path, "new", newText),
				},
			)
		} catch {
			return null
		}
	}, [path, oldText, newText])

	if (!fileDiff) return null

	return (
		<PierreDiffFrame
			title={fill ? undefined : path}
			className={className}
			expanded={expanded}
			fill={fill}
		>
			<FileDiff
				fileDiff={fileDiff}
				options={PIERRE_DIFF_OPTIONS}
				disableWorkerPool
				className="pierre-diff-host"
			/>
		</PierreDiffFrame>
	)
}

interface PierrePatchDiffProps {
	patch: string
	title?: string
	className?: string
}

export function PierrePatchDiff({ patch, title, className }: PierrePatchDiffProps) {
	const normalizedPatch = useMemo(
		() => ensureUnifiedDiffFormat(patch, title ?? "patch"),
		[patch, title],
	)

	return (
		<PierreDiffFrame title={title} className={className}>
			<PatchDiff
				patch={normalizedPatch}
				options={PIERRE_DIFF_OPTIONS}
				disableWorkerPool
				className="pierre-diff-host"
			/>
		</PierreDiffFrame>
	)
}