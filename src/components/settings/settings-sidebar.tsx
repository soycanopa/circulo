import { ArrowLeft } from "lucide-react"
import { WindowChromeControls } from "@/components/layout/window-chrome-controls"
import {
	SETTINGS_SECTIONS,
	type SettingsSectionId,
} from "@/lib/settings-sections"
import { cn } from "@/lib/utils"

interface SettingsSidebarProps {
	activeSection: SettingsSectionId
	onSelectSection: (section: SettingsSectionId) => void
	onClose: () => void
	onHideSidebar: () => void
}

export function SettingsSidebar({
	activeSection,
	onSelectSection,
	onClose,
	onHideSidebar,
}: SettingsSidebarProps) {
	return (
		<div className="flex h-full min-h-0 flex-col">
			<div
				className="flex h-12 shrink-0 items-center border-b border-border pb-0.5"
				data-tauri-drag-region="deep"
			>
				<WindowChromeControls
					sidebarOpen
					layout="sidebar"
					onToggleSidebar={onHideSidebar}
				/>
			</div>

			<div className="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto p-3">
				<button
					type="button"
					onClick={onClose}
					className="flex shrink-0 items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm font-medium text-fg transition hover:bg-white/[0.06]"
					title="Back to chats (Esc)"
				>
					<ArrowLeft className="size-4 shrink-0" />
					Back to chats
				</button>

				<p className="px-2.5 pb-0.5 pt-4 text-[11px] font-medium tracking-tight text-muted/55">
					Settings
				</p>

				<div className="flex flex-col gap-0.5">
					{SETTINGS_SECTIONS.map((section) => {
						const active = section.id === activeSection
						const Icon = section.icon
						return (
							<button
								key={section.id}
								type="button"
								onClick={() => onSelectSection(section.id)}
								className={cn(
									"flex items-start gap-2 rounded-md px-2.5 py-1.5 text-left transition",
									active
										? "bg-white/10 text-fg"
										: "text-fg/80 hover:bg-white/[0.06]",
								)}
							>
								<Icon
									className={cn(
										"mt-0.5 size-4 shrink-0",
										active ? "text-accent" : "text-muted",
									)}
								/>
								<span className="min-w-0 flex-1">
									<span className="block truncate text-xs font-medium">
										{section.label}
									</span>
									<span className="mt-0.5 block truncate text-[10px] leading-snug text-muted">
										{section.description}
									</span>
								</span>
							</button>
						)
					})}
				</div>
			</div>
		</div>
	)
}
