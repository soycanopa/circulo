import {
	File,
	FileCode2,
	FileImage,
	FileJson2,
	FileText,
	Loader2,
	type LucideIcon,
} from "lucide-react"
import { ComposerCommandMenu } from "@/components/chat/composer-command-menu"
import {
	CommandEmpty,
	CommandGroup,
	CommandItem,
	CommandList,
	CommandShortcut,
} from "@/components/ui/command"

interface FileMentionPickerProps {
	query: string
	files: string[]
	isLoading: boolean
	hasProject: boolean
	selectedValue: string
	onSelect: (path: string) => void
	onValueChange: (value: string) => void
}

function fileIcon(path: string): LucideIcon {
	const name = path.split("/").pop() ?? path
	const extension = name.includes(".") ? name.split(".").pop()?.toLowerCase() : ""

	switch (extension) {
		case "ts":
		case "tsx":
		case "js":
		case "jsx":
		case "rs":
		case "go":
		case "py":
		case "zig":
		case "swift":
		case "css":
		case "scss":
			return FileCode2
		case "json":
		case "jsonc":
		case "yaml":
		case "yml":
		case "toml":
			return FileJson2
		case "png":
		case "jpg":
		case "jpeg":
		case "gif":
		case "webp":
		case "svg":
			return FileImage
		case "md":
		case "mdx":
		case "txt":
			return FileText
		default:
			return File
	}
}

function formatFileLabel(path: string) {
	const segments = path.split("/")
	const fileName = segments.pop() ?? path
	const directory = segments.join("/")
	return { fileName, directory }
}

export function FileMentionPicker({
	query,
	files,
	isLoading,
	hasProject,
	selectedValue,
	onSelect,
	onValueChange,
}: FileMentionPickerProps) {
	const emptyMessage = !hasProject
		? "Abre un proyecto para buscar archivos."
		: query.trim()
			? "No hay archivos en el proyecto que coincidan."
			: "Escribe para filtrar archivos del proyecto."

	return (
		<ComposerCommandMenu
			value={selectedValue}
			onValueChange={onValueChange}
			query={query ? `@${query}` : "@"}
			placeholder="Buscar archivos del proyecto…"
		>
			<CommandList>
				{isLoading ? (
					<div className="flex items-center gap-2 px-3 py-6 text-sm text-muted-foreground">
						<Loader2 className="size-4 animate-spin" />
						Buscando archivos…
					</div>
				) : null}
				{!isLoading ? <CommandEmpty>{emptyMessage}</CommandEmpty> : null}
				{!isLoading && files.length > 0 ? (
					<CommandGroup heading="Archivos del proyecto">
						{files.map((path) => {
							const Icon = fileIcon(path)
							const { fileName, directory } = formatFileLabel(path)
							return (
								<CommandItem
									key={path}
									value={path}
									onSelect={() => onSelect(path)}
									title={path}
								>
									<Icon />
									<span className="truncate">{fileName}</span>
									{directory ? (
										<CommandShortcut className="max-w-[45%] tracking-normal">
											{directory}
										</CommandShortcut>
									) : null}
								</CommandItem>
							)
						})}
					</CommandGroup>
				) : null}
			</CommandList>
		</ComposerCommandMenu>
	)
}