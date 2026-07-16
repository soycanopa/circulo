import {
	ArrowLeft,
	Info,
	Keyboard,
	Plug,
	Settings2,
	Sparkles,
	User,
} from "lucide-react"
import type { LucideIcon } from "lucide-react"
import { useAtom, useSetAtom } from "jotai"
import {
	Sidebar,
	SidebarContent,
	SidebarGroup,
	SidebarMenu,
	SidebarMenuButton,
	SidebarMenuItem,
} from "@/components/layout/sidebar-layout"
import { SETTINGS_SECTIONS, type SettingsSection } from "@/lib/app-settings"
import { settingsOpenAtom, settingsSectionAtom } from "@/stores/atoms"

const SECTION_ICONS: Record<SettingsSection, LucideIcon> = {
	general: Settings2,
	profile: User,
	shortcuts: Keyboard,
	skills: Sparkles,
	mcp: Plug,
	about: Info,
}

export function SettingsSidebar() {
	const [section, setSection] = useAtom(settingsSectionAtom)
	const setSettingsOpen = useSetAtom(settingsOpenAtom)

	return (
		<Sidebar>
			<SidebarContent>
				<SidebarGroup>
					<SidebarMenu>
						<SidebarMenuItem>
							<SidebarMenuButton
								onClick={() => setSettingsOpen(false)}
								className="text-[#FAFAFA]"
							>
								<ArrowLeft className="size-4" />
								<span>Back to chat</span>
							</SidebarMenuButton>
						</SidebarMenuItem>
					</SidebarMenu>
				</SidebarGroup>

				<SidebarGroup label="Settings">
					<SidebarMenu>
						{SETTINGS_SECTIONS.map((entry) => {
							const Icon = SECTION_ICONS[entry.id]
							return (
								<SidebarMenuItem key={entry.id}>
									<SidebarMenuButton
										isActive={section === entry.id}
										onClick={() => setSection(entry.id)}
										className="text-[#FAFAFA]"
									>
										<Icon className="size-4 shrink-0 opacity-80" />
										<span className="truncate">{entry.label}</span>
									</SidebarMenuButton>
								</SidebarMenuItem>
							)
						})}
					</SidebarMenu>
				</SidebarGroup>
			</SidebarContent>
		</Sidebar>
	)
}