import type { ReactNode } from "react"
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