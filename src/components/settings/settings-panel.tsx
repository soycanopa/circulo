import { AboutSettings } from "@/components/settings/sections/about-settings"
import { AgentsSettings } from "@/components/settings/sections/agents-settings"
import { GeneralSettings } from "@/components/settings/sections/general-settings"
import { McpSettings } from "@/components/settings/sections/mcp-settings"
import { ProfileSettings } from "@/components/settings/sections/profile-settings"
import { ShortcutsSettings } from "@/components/settings/sections/shortcuts-settings"
import { SkillsSettings } from "@/components/settings/sections/skills-settings"
import type { SettingsSection } from "@/lib/app-settings"

export function SettingsPanel({ section }: { section: SettingsSection }) {
	switch (section) {
		case "general":
			return <GeneralSettings />
		case "agents":
			return <AgentsSettings />
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