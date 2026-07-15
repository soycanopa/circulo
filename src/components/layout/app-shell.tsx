import { open } from "@tauri-apps/plugin-dialog"
import { FolderOpen, Hammer, Power } from "lucide-react"
import { useAtom } from "jotai"
import { useEffect, useState } from "react"
import { ChatView } from "@/components/chat/chat-view"
import { Button } from "@/components/ui/button"
import { closeProject, getProjectStatus, openProject } from "@/lib/tauri"
import { projectPathAtom } from "@/stores/atoms"

export function AppShell() {
	const [projectPath, setProjectPath] = useAtom(projectPathAtom)
	const [connected, setConnected] = useState(false)
	const [loading, setLoading] = useState(false)

	useEffect(() => {
		getProjectStatus()
			.then((status) => {
				setConnected(status.connected)
				setProjectPath(status.projectPath)
			})
			.catch(() => {
				setConnected(false)
			})
	}, [setProjectPath])

	async function handleOpenProject() {
		const selected = await open({
			directory: true,
			multiple: false,
			title: "Abrir proyecto",
		})

		if (!selected || Array.isArray(selected)) return

		setLoading(true)
		try {
			const status = await openProject(selected)
			setConnected(status.connected)
			setProjectPath(status.projectPath)
		} finally {
			setLoading(false)
		}
	}

	async function handleCloseProject() {
		setLoading(true)
		try {
			const status = await closeProject()
			setConnected(status.connected)
			setProjectPath(status.projectPath)
		} finally {
			setLoading(false)
		}
	}

	return (
		<div className="flex h-screen bg-background text-foreground">
			<aside className="flex w-72 shrink-0 flex-col border-r border-border bg-card">
				<div className="flex items-center gap-2 border-b border-border px-4 py-4">
					<Hammer className="size-5 text-ring" />
					<div>
						<p className="text-sm font-semibold">Forge</p>
						<p className="text-xs text-muted-foreground">Orquestador ACP</p>
					</div>
				</div>

				<div className="flex flex-col gap-2 p-4">
					<Button onClick={() => void handleOpenProject()} disabled={loading}>
						<FolderOpen className="mr-2 size-4" />
						Abrir proyecto
					</Button>
					<Button
						variant="secondary"
						onClick={() => void handleCloseProject()}
						disabled={!connected || loading}
					>
						<Power className="mr-2 size-4" />
						Cerrar sesión
					</Button>
				</div>

				<div className="mt-auto border-t border-border p-4 text-xs text-muted-foreground">
					<p className="font-medium text-foreground">Agente activo</p>
					<p className="mt-1 font-mono">opencode acp</p>
					{projectPath ? (
						<p className="mt-3 break-all font-mono text-[11px]">{projectPath}</p>
					) : (
						<p className="mt-3">Ningún proyecto abierto</p>
					)}
				</div>
			</aside>

			<main className="min-w-0 flex-1">
				<ChatView connected={connected} />
			</main>
		</div>
	)
}