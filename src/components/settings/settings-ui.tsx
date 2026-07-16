import { ChevronRight } from "lucide-react"
import type { ReactNode } from "react"
import { useId, useState } from "react"
import { cn } from "@/lib/utils"

export function SettingsSectionHeader({
	title,
	description,
}: {
	title: string
	description?: string
}) {
	return (
		<div className="mb-4">
			<h3 className="text-sm font-medium text-foreground">{title}</h3>
			{description ? (
				<p className="mt-1 text-xs text-muted-foreground">{description}</p>
			) : null}
		</div>
	)
}

export function SettingsGroup({ children }: { children: ReactNode }) {
	return (
		<div className="divide-y divide-border/60 rounded-lg border border-border/60 bg-muted/20">
			{children}
		</div>
	)
}

export function SettingsRow({
	label,
	description,
	children,
	className,
}: {
	label: string
	description?: string
	children: ReactNode
	className?: string
}) {
	return (
		<div className={cn("flex items-start justify-between gap-4 px-4 py-3", className)}>
			<div className="min-w-0 flex-1">
				<p className="text-sm text-foreground">{label}</p>
				{description ? (
					<p className="mt-0.5 text-xs text-muted-foreground">{description}</p>
				) : null}
			</div>
			<div className="shrink-0">{children}</div>
		</div>
	)
}

export function SettingsSelect({
	value,
	onChange,
	options,
	disabled,
}: {
	value: string
	onChange: (value: string) => void
	options: { value: string; label: string; disabled?: boolean }[]
	disabled?: boolean
}) {
	return (
		<select
			value={value}
			disabled={disabled}
			onChange={(event) => onChange(event.target.value)}
			className="h-8 max-w-[220px] rounded-md border border-border bg-background px-2 text-xs text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
		>
			{options.map((option) => (
				<option key={option.value} value={option.value} disabled={option.disabled}>
					{option.label}
				</option>
			))}
		</select>
	)
}

export function SettingsToggle({
	checked,
	onChange,
	disabled,
	ariaLabel,
}: {
	checked: boolean
	onChange: (checked: boolean) => void
	disabled?: boolean
	ariaLabel: string
}) {
	return (
		<button
			type="button"
			role="switch"
			aria-checked={checked}
			aria-label={ariaLabel}
			disabled={disabled}
			onClick={() => onChange(!checked)}
			className={cn(
				"relative h-5 w-9 rounded-full transition-colors disabled:cursor-not-allowed disabled:opacity-50",
				checked ? "bg-primary" : "bg-muted",
			)}
		>
			<span
				className={cn(
					"absolute top-0.5 size-4 rounded-full bg-background shadow transition-transform",
					checked ? "left-[18px]" : "left-0.5",
				)}
			/>
		</button>
	)
}

export function SettingsEmptyState({ children }: { children: ReactNode }) {
	return (
		<p className="rounded-lg border border-dashed border-border/60 px-4 py-6 text-center text-xs text-muted-foreground">
			{children}
		</p>
	)
}

export function SettingsBadge({
	children,
	tone = "neutral",
}: {
	children: ReactNode
	tone?: "neutral" | "accent" | "success" | "muted"
}) {
	return (
		<span
			className={cn(
				"inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-medium tabular-nums",
				tone === "accent" && "bg-primary/15 text-primary",
				tone === "success" && "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
				tone === "muted" && "bg-muted text-muted-foreground",
				tone === "neutral" && "bg-background text-muted-foreground ring-1 ring-border/60",
			)}
		>
			{children}
		</span>
	)
}

export function SettingsCollapsible({
	title,
	subtitle,
	badges,
	icon,
	level = "primary",
	defaultOpen = false,
	open: controlledOpen,
	onOpenChange,
	children,
}: {
	title: string
	subtitle?: string
	badges?: ReactNode
	icon?: ReactNode
	level?: "primary" | "nested"
	defaultOpen?: boolean
	open?: boolean
	onOpenChange?: (open: boolean) => void
	children: ReactNode
}) {
	const contentId = useId()
	const [uncontrolledOpen, setUncontrolledOpen] = useState(defaultOpen)
	const isControlled = controlledOpen !== undefined
	const open = isControlled ? controlledOpen : uncontrolledOpen

	function setOpen(next: boolean) {
		if (!isControlled) setUncontrolledOpen(next)
		onOpenChange?.(next)
	}

	return (
		<div
			className={cn(
				level === "primary" && "overflow-hidden rounded-xl border border-border/60 bg-muted/15",
				level === "nested" && "rounded-lg border border-border/40 bg-background/40",
			)}
		>
			<button
				type="button"
				aria-expanded={open}
				aria-controls={contentId}
				onClick={() => setOpen(!open)}
				className={cn(
					"flex w-full items-center gap-3 text-left transition-colors hover:bg-muted/30",
					level === "primary" ? "px-4 py-3" : "px-3 py-2.5",
				)}
			>
				<ChevronRight
					className={cn(
						"size-3.5 shrink-0 text-muted-foreground transition-transform duration-200 ease-out motion-reduce:transition-none",
						open && "rotate-90",
					)}
				/>
				{icon ? <span className="shrink-0 text-muted-foreground">{icon}</span> : null}
				<span className="min-w-0 flex-1">
					<span
						className={cn(
							"block truncate text-foreground",
							level === "primary" ? "text-sm font-medium" : "text-xs font-medium",
						)}
					>
						{title}
					</span>
					{subtitle ? (
						<span className="mt-0.5 block truncate text-[11px] text-muted-foreground">
							{subtitle}
						</span>
					) : null}
				</span>
				{badges ? <span className="flex shrink-0 flex-wrap items-center gap-1">{badges}</span> : null}
			</button>
			<div
				id={contentId}
				className={cn(
					"grid transition-[grid-template-rows] duration-200 ease-out motion-reduce:transition-none",
					open ? "grid-rows-[1fr]" : "grid-rows-[0fr]",
				)}
			>
				<div className="overflow-hidden">
					<div
						className={cn(
							level === "primary" && "border-t border-border/50 px-2 pb-2 pt-1",
							level === "nested" && "border-t border-border/40 px-2 pb-2 pt-1",
						)}
					>
						{children}
					</div>
				</div>
			</div>
		</div>
	)
}