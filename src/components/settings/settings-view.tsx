import { useEffect, type ReactNode } from "react"
import { WindowChromeControls } from "@/components/layout/window-chrome-controls"
import { SETTINGS_SECTIONS, type SettingsSectionId } from "@/lib/settings-sections"
import { cn } from "@/lib/utils"

function isTypingTarget(target: EventTarget | null): boolean {
	if (!(target instanceof HTMLElement)) return false
	const tag = target.tagName
	return (
		target.isContentEditable ||
		tag === "INPUT" ||
		tag === "TEXTAREA" ||
		tag === "SELECT"
	)
}

interface SettingsViewProps {
	activeSection: SettingsSectionId
	onClose: () => void
	sidebarVisible: boolean
	onToggleSidebar: () => void
	children: ReactNode
}

export function SettingsView({
	activeSection,
	onClose,
	sidebarVisible,
	onToggleSidebar,
	children,
}: SettingsViewProps) {
	const section = SETTINGS_SECTIONS.find((s) => s.id === activeSection)

	useEffect(() => {
		function onKey(event: KeyboardEvent) {
			if (event.key !== "Escape") return
			if (isTypingTarget(event.target)) return
			event.preventDefault()
			onClose()
		}
		window.addEventListener("keydown", onKey, { capture: true })
		return () => window.removeEventListener("keydown", onKey, { capture: true })
	}, [onClose])

	return (
		<div className="flex h-full min-h-0 flex-col">
			<div
				className="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-border pb-0.5"
				data-tauri-drag-region="deep"
			>
				<div className="flex min-w-0 flex-1 items-center gap-2">
					{!sidebarVisible ? (
						<WindowChromeControls
							sidebarOpen={false}
							layout="inline"
							onToggleSidebar={onToggleSidebar}
						/>
					) : null}
					<span
						className={cn(
							"min-w-0 flex-1 truncate text-xs text-muted",
							sidebarVisible ? "px-4" : "pl-2 pr-4",
						)}
					>
						Settings — {section?.label ?? "General"}
					</span>
				</div>
				<div className="flex shrink-0 items-center gap-2 pr-4">
					<button
						type="button"
						onClick={onClose}
						className="rounded-md border border-border px-2.5 py-1 text-[11px] text-fg transition hover:bg-white/5"
						data-tauri-drag-region="false"
					>
						Done
					</button>
				</div>
			</div>

			<div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
				<div className="mx-auto w-full max-w-2xl px-6 py-6">{children}</div>
			</div>
		</div>
	)
}
