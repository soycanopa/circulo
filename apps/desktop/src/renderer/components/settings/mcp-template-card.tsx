import { Badge } from "@circulo/ui/components/badge"
import { Button } from "@circulo/ui/components/button"
import { Input } from "@circulo/ui/components/input"
import { Tooltip, TooltipContent, TooltipTrigger } from "@circulo/ui/components/tooltip"
import {
	BookOpenIcon,
	BoxIcon,
	CheckCircle2Icon,
	DatabaseIcon,
	FileTextIcon,
	FolderGitIcon,
	FolderTreeIcon,
	GlobeIcon,
	Grid2x2Icon,
	ImageIcon,
	LayoutIcon,
	Link2Icon,
	PaintbrushIcon,
	PaletteIcon,
	PencilRulerIcon,
	PenToolIcon,
	PlayIcon,
	TriangleIcon,
	UsersIcon,
	WandIcon,
	XCircleIcon,
	BotIcon,
	type LucideIcon,
} from "lucide-react"
import { useState } from "react"
import type { InstallState } from "../../hooks/use-mcp-install"
import type { McpTemplate } from "./mcp-templates"

// ============================================================
// Icon mapping
// ============================================================

const iconMap: Record<string, LucideIcon> = {
	paintbrush: PaintbrushIcon,
	github: FolderGitIcon,
	"folder-tree": FolderTreeIcon,
	layout: LayoutIcon,
	wand: WandIcon,
	"pencil-ruler": PencilRulerIcon,
	database: DatabaseIcon,
	triangle: TriangleIcon,
	"pen-tool": PenToolIcon,
	"file-text": FileTextIcon,
	"book-open": BookOpenIcon,
	users: UsersIcon,
	"grid-2x2": Grid2x2Icon,
	globe: GlobeIcon,
	"link-2": Link2Icon,
	palette: PaletteIcon,
	box: BoxIcon,
	image: ImageIcon,
	play: PlayIcon,
}

function getIcon(name: string): LucideIcon {
	return iconMap[name] ?? WandIcon
}

// ============================================================
// Env var form (unchanged)
// ============================================================

function EnvVarForm({
	template,
	onSubmit,
	onCancel,
}: {
	template: McpTemplate
	onSubmit: (values: Record<string, string>) => void
	onCancel: () => void
}) {
	const [values, setValues] = useState<Record<string, string>>({})

	return (
		<div className="space-y-2">
			{template.envVars?.map((ev) => (
				<div key={ev.key} className="space-y-1">
					<div className="flex items-center justify-between">
						<label
							htmlFor={`env-${template.id}-${ev.key}`}
							className="text-xs font-medium"
						>
							{ev.label}
						</label>
						{ev.helpUrl && (
							<a
								href={ev.helpUrl}
								target="_blank"
								rel="noreferrer"
								className="text-xs text-muted-foreground underline"
							>
								Get token
							</a>
						)}
					</div>
					<Input
						id={`env-${template.id}-${ev.key}`}
						type="password"
						placeholder={`Enter ${ev.label}`}
						value={values[ev.key] ?? ""}
						onChange={(e) =>
							setValues((prev) => ({ ...prev, [ev.key]: e.target.value }))
						}
						className="h-8 text-xs"
					/>
				</div>
			))}
			<div className="flex gap-2 pt-1">
				<Button size="sm" className="h-7 text-xs" onClick={() => onSubmit(values)}>
					Install
				</Button>
				<Button size="sm" variant="ghost" className="h-7 text-xs" onClick={onCancel}>
					Cancel
				</Button>
			</div>
		</div>
	)
}

// ============================================================
// Card component (clean, no progress bar)
// ============================================================

interface McpTemplateCardProps {
	template: McpTemplate
	state: InstallState
	installed: boolean
	onInstall: (template: McpTemplate, envValues?: Record<string, string>) => void
	onCancel: () => void
}

export function McpTemplateCard({
	template,
	state,
	installed,
	onInstall,
	onCancel,
}: McpTemplateCardProps) {
	const [showEnvForm, setShowEnvForm] = useState(false)
	const Icon = getIcon(template.icon)
	const isInstalling = state.status === "installing"
	const isFailed = state.status === "failed"
	const isDone = state.status === "connected" || installed

	const handleInstallClick = () => {
		if (template.envVars && template.envVars.length > 0 && template.type === "simple") {
			setShowEnvForm(true)
		} else {
			onInstall(template)
		}
	}

	const handleEnvSubmit = (values: Record<string, string>) => {
		setShowEnvForm(false)
		onInstall(template, values)
	}

	const errorMessage = isFailed ? (state.error ?? "Installation failed") : null

	return (
		<div className="flex flex-col rounded-lg border p-2.5 transition-colors">
			{/* Header row: icon + name + status */}
			<div className="flex items-center gap-2">
				<Icon aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
				<span className="truncate text-xs font-medium">{template.name}</span>

				{template.type === "agent" && (
					<Badge variant="secondary" className="h-4 gap-0.5 px-1 py-0 text-[10px]">
						<BotIcon aria-hidden="true" className="size-2.5" />
						Agent
					</Badge>
				)}

				{/* Status icons (right-aligned) */}
				<div className="ml-auto flex items-center gap-1">
					{isDone && (
						<CheckCircle2Icon
							aria-hidden="true"
							className="size-3.5 shrink-0 text-green-500"
						/>
					)}
					{isFailed && (
						<Tooltip>
							<TooltipTrigger
								render={
									<span className="inline-flex cursor-default" />
								}
							>
								<XCircleIcon
									aria-hidden="true"
									className="size-3.5 shrink-0 text-destructive"
								/>
							</TooltipTrigger>
							<TooltipContent className="max-w-xs bg-zinc-800 text-zinc-100">
								<p className="text-xs text-red-400">{errorMessage}</p>
							</TooltipContent>
						</Tooltip>
					)}
				</div>
			</div>

			{/* Env var form */}
			{showEnvForm && !isInstalling && (
				<div className="mt-2">
					<EnvVarForm
						template={template}
						onSubmit={handleEnvSubmit}
						onCancel={() => setShowEnvForm(false)}
					/>
				</div>
			)}

			{/* Installed done state */}
			{isDone && !isInstalling && (
				<p className="mt-1 text-[11px] text-muted-foreground">Installed &amp; ready</p>
			)}

			{/* Action buttons */}
			<div className="mt-2">
				{!isInstalling && !isFailed && !isDone && !showEnvForm && (
					<Button
						size="sm"
						variant="outline"
						className="h-6 w-full text-[11px]"
						onClick={handleInstallClick}
					>
						Install
					</Button>
				)}
				{isFailed && (
					<div className="space-y-1.5">
						<Button
							size="sm"
							variant="outline"
							className="h-6 flex-1 text-[11px]"
							onClick={handleInstallClick}
						>
							Retry
						</Button>
						<Button
							size="sm"
							variant="ghost"
							className="h-6 text-[11px]"
							onClick={onCancel}
						>
							Dismiss
						</Button>
					</div>
				)}
				{isInstalling && (
					<div className="flex items-center justify-between">
						<span className="flex items-center gap-1 truncate text-[10px] text-muted-foreground">
							Installing...
						</span>
						<Button
							size="sm"
							variant="ghost"
							className="h-6 text-[11px]"
							onClick={onCancel}
						>
							Cancel
						</Button>
					</div>
				)}
			</div>
		</div>
	)
}
