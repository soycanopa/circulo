import { useEffect, useState } from "react"
import {
	AutoEditSwitch,
	PathRow,
	SectionHeader,
	SettingRow,
} from "@/components/settings/sections/section-ui"
import { getDefaultChatsPath } from "@/lib/tauri"

export function GeneralSection() {
	const [chatsPath, setChatsPath] = useState<string | null>(null)

	useEffect(() => {
		void getDefaultChatsPath().then(setChatsPath)
	}, [])

	return (
		<div>
			<SectionHeader
				title="General"
				description="Core behavior and where your chats are stored."
			/>
			<div className="space-y-3">
				<SettingRow
					label="Auto-edit"
					description="The agent can edit files without asking for permission on every change."
					control={<AutoEditSwitch />}
				/>
				<PathRow label="Chats folder" path={chatsPath} />
			</div>
		</div>
	)
}
