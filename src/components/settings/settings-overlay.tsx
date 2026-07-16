import { useAtom } from "jotai"
import { useState } from "react"
import { OverlayShell } from "@/components/overlays/overlay-shell"
import { AboutSettings } from "@/components/settings/sections/about-settings"
import { GeneralSettings } from "@/components/settings/sections/general-settings"
import { McpSettings } from "@/components/settings/sections/mcp-settings"
import { ProfileSettings } from "@/components/settings/sections/profile-settings"
import { ShortcutsSettings } from "@/components/settings/sections/shortcuts-settings"
import { SkillsSettings } from "@/components/settings/sections/skills-settings"
import { SETTINGS_SECTIONS, type SettingsSection } from "@/lib/app-settings"
import { cn } from "@/lib/utils"
import { settingsOpenAtom } from "@/stores/atoms"

function SettingsPanel({ section }: { section: SettingsSection }) {
	switch (section) {
		case "general":
			return <GeneralSettings />
		case "profile":
			return <ProfileSettings />
		case "shortcuts":
			return <ShortcutsSettings />
		case "skills":
			return <SkillsSettings />
		case "mcp":
			return <McpSettings />
		case "about":
			return <AboutSettings />
		default:
			return null
	}
}

export function SettingsOverlay() {
	const [open, setOpen] = useAtom(settingsOpenAtom)
	const [section, setSection] = useState<SettingsSection>("general")

	return (
		<OverlayShell
			open={open}
			title="Settings"
			subtitle="Circulo preferences"
			onClose={() => setOpen(false)}
			className="max-w-4xl"
		>
			<div className="flex min-h-[min(70vh,640px)]">
				<nav className="w-44 shrink-0 border-r border-border/60 bg-muted/10 p-2">
					<ul className="flex flex-col gap-0.5">
						{SETTINGS_SECTIONS.map((entry) => (
							<li key={entry.id}>
								<button
									type="button"
									onClick={() => setSection(entry.id)}
									className={cn(
										"w-full rounded-md px-3 py-2 text-left text-xs transition-colors",
										section === entry.id
											? "bg-accent text-foreground"
											: "text-muted-foreground hover:bg-accent/60 hover:text-foreground",
									)}
								>
									{entry.label}
								</button>
							</li>
						))}
					</ul>
				</nav>
				<div className="scrollbar-thin min-w-0 flex-1 overflow-y-auto p-5">
					<SettingsPanel section={section} />
				</div>
			</div>
		</OverlayShell>
	)
}