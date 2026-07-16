import { Command as CommandPrimitive } from "cmdk"
import type { ComponentProps } from "react"
import { cn } from "@/lib/utils"

export function Command({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive>) {
	return (
		<CommandPrimitive
			className={cn(
				"flex h-full w-full flex-col overflow-hidden rounded-lg bg-popover text-popover-foreground",
				className,
			)}
			{...props}
		/>
	)
}

export function CommandList({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive.List>) {
	return (
		<CommandPrimitive.List
			className={cn("max-h-64 overflow-y-auto overflow-x-hidden scrollbar-thin", className)}
			{...props}
		/>
	)
}

export function CommandEmpty({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive.Empty>) {
	return (
		<CommandPrimitive.Empty
			className={cn("px-3 py-6 text-center text-sm text-muted-foreground", className)}
			{...props}
		/>
	)
}

export function CommandGroup({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive.Group>) {
	return (
		<CommandPrimitive.Group
			className={cn(
				"overflow-hidden p-1 text-foreground [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[10px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:uppercase [&_[cmdk-group-heading]]:tracking-wide [&_[cmdk-group-heading]]:text-muted-foreground",
				className,
			)}
			{...props}
		/>
	)
}

export function CommandItem({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive.Item>) {
	return (
		<CommandPrimitive.Item
			className={cn(
				"relative flex cursor-default select-none items-center gap-2 rounded-md px-2 py-2 text-sm outline-none data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-50 data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground",
				className,
			)}
			{...props}
		/>
	)
}

export function CommandSeparator({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive.Separator>) {
	return (
		<CommandPrimitive.Separator
			className={cn("-mx-1 h-px bg-border", className)}
			{...props}
		/>
	)
}