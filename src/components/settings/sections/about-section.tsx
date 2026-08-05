import { useEffect, useState } from "react"
import { SectionHeader } from "@/components/settings/sections/section-ui"
import { PathRow, SettingRow } from "@/components/settings/sections/section-ui"
import { getDefaultChatsPath, getHomePath } from "@/lib/tauri"

const APP_VERSION = "0.4.0"

export function AboutSection() {
	const [homePath, setHomePath] = useState<string | null>(null)
	const [chatsPath, setChatsPath] = useState<string | null>(null)

	useEffect(() => {
		void getHomePath().then(setHomePath)
		void getDefaultChatsPath().then(setChatsPath)
	}, [])

	return (
		<div>
			<SectionHeader
				title="About"
				description="Version and where Circulo keeps your data."
			/>
			<div className="space-y-3">
				<SettingRow
					label="Version"
					description="Desktop ACP client"
					control={<code className="text-xs text-fg/90">v{APP_VERSION}</code>}
				/>
				<PathRow label="Home directory" path={homePath} />
				<PathRow label="Chats folder" path={chatsPath} />
				<p className="text-[11px] leading-snug text-muted">
					Settings are stored in{" "}
					<code className="rounded bg-white/5 px-1">~/.circulo/config.json</code>{" "}
					and chat transcripts live under the chats folder, one folder per
					project.
				</p>
			</div>
		</div>
	)
}
