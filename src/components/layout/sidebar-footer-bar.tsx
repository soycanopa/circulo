import { Settings } from "lucide-react"
import { useSetAtom } from "jotai"
import { ConnectionStatus } from "@/components/layout/connection-status"
import { ProfileAvatar } from "@/components/profile/profile-avatar"
import { useProfileIdentity } from "@/lib/profile-identity"
import { cn } from "@/lib/utils"
import { settingsOpenAtom, settingsSectionAtom } from "@/stores/atoms"

interface SidebarFooterBarProps {
	connected: boolean
	loading?: boolean
}

export function SidebarFooterBar({ connected, loading = false }: SidebarFooterBarProps) {
	const { name, handle, color, image, initials, configured } = useProfileIdentity()
	const setSettingsOpen = useSetAtom(settingsOpenAtom)
	const setSettingsSection = useSetAtom(settingsSectionAtom)

	function openSettings(section: "general" | "profile" = "general") {
		setSettingsSection(section)
		setSettingsOpen(true)
	}

	return (
		<div className="flex w-full min-w-0 items-center justify-between gap-1">
			{configured ? (
				<button
					type="button"
					disabled={loading}
					onClick={() => openSettings("profile")}
					className={cn(
						"flex min-w-0 flex-1 items-center gap-2.5 rounded-md px-1 py-1 text-left transition-colors",
						"hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
						"disabled:cursor-not-allowed disabled:opacity-50",
					)}
					title="Ver perfil"
				>
					<ProfileAvatar
						initials={initials}
						color={color}
						image={image}
						className="size-8 shrink-0"
						textClassName="text-[11px]"
					/>
					<div className="min-w-0 flex-1 leading-tight">
						<p className="truncate text-sm font-medium text-sidebar-foreground">{name}</p>
						<p className="truncate text-[11px] text-muted-foreground">{handle}</p>
					</div>
				</button>
			) : (
				<ConnectionStatus connected={connected} />
			)}

			<div className="flex shrink-0 items-center">
				{configured ? <ConnectionStatus connected={connected} /> : null}
				<button
					type="button"
					disabled={loading}
					onClick={() => openSettings("general")}
					className={cn(
						"flex size-8 items-center justify-center rounded-md text-sidebar-foreground/70 transition-colors",
						"hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
						"disabled:cursor-not-allowed disabled:opacity-50",
					)}
					aria-label="Settings"
					title="Settings"
				>
					<Settings className="size-4 shrink-0" />
				</button>
			</div>
		</div>
	)
}