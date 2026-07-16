import { useCallback, useEffect, useState } from "react"
import { buildSlashEntries, type SlashEntry } from "@/lib/slash-prompt"
import {
	listOpencodeCommands,
	listOpencodeMcpServers,
	listOpencodeSkills,
	type OpencodeCommandEntry,
	type OpencodeMcpServerEntry,
	type OpencodeSkillEntry,
} from "@/lib/tauri"

export function useSlashEntries(projectPath: string | null) {
	const [entries, setEntries] = useState<SlashEntry[]>([])
	const [commands, setCommands] = useState<OpencodeCommandEntry[]>([])
	const [skills, setSkills] = useState<OpencodeSkillEntry[]>([])
	const [mcpServers, setMcpServers] = useState<OpencodeMcpServerEntry[]>([])
	const [loading, setLoading] = useState(true)

	const refresh = useCallback(async () => {
		setLoading(true)
		try {
			const [commandRows, skillRows, mcpRows] = await Promise.all([
				listOpencodeCommands(projectPath),
				listOpencodeSkills(projectPath),
				listOpencodeMcpServers(projectPath),
			])
			setCommands(commandRows)
			setSkills(skillRows)
			setMcpServers(mcpRows)
			setEntries(buildSlashEntries(commandRows, skillRows, mcpRows))
		} catch {
			setCommands([])
			setSkills([])
			setMcpServers([])
			setEntries([])
		} finally {
			setLoading(false)
		}
	}, [projectPath])

	useEffect(() => {
		void refresh()
	}, [refresh])

	useEffect(() => {
		const handler = () => void refresh()
		window.addEventListener("circulo:mcp-changed", handler)
		return () => window.removeEventListener("circulo:mcp-changed", handler)
	}, [refresh])

	return { entries, commands, skills, mcpServers, loading, refresh }
}