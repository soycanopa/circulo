import { useCallback, useEffect, useState } from "react"
import { buildSlashEntries, type SlashEntry } from "@/lib/slash-prompt"
import {
	listOpencodeCommands,
	listOpencodeSkills,
	type OpencodeCommandEntry,
	type OpencodeSkillEntry,
} from "@/lib/tauri"

export function useSlashEntries(projectPath: string | null) {
	const [entries, setEntries] = useState<SlashEntry[]>([])
	const [commands, setCommands] = useState<OpencodeCommandEntry[]>([])
	const [skills, setSkills] = useState<OpencodeSkillEntry[]>([])
	const [loading, setLoading] = useState(true)

	const refresh = useCallback(async () => {
		setLoading(true)
		try {
			const [commandRows, skillRows] = await Promise.all([
				listOpencodeCommands(projectPath),
				listOpencodeSkills(projectPath),
			])
			setCommands(commandRows)
			setSkills(skillRows)
			setEntries(buildSlashEntries(commandRows, skillRows))
		} catch {
			setCommands([])
			setSkills([])
			setEntries([])
		} finally {
			setLoading(false)
		}
	}, [projectPath])

	useEffect(() => {
		void refresh()
	}, [refresh])

	return { entries, commands, skills, loading, refresh }
}