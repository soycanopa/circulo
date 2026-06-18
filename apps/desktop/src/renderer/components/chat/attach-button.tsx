import {
	PromptInputButton,
	usePromptInputAttachments,
} from "@circulo/ui/components/ai-elements/prompt-input"
import { PlusIcon } from "lucide-react"

export const ACCEPTED_FILE_TYPES =
	"image/png,image/jpeg,image/gif,image/webp,application/pdf,.md,.txt,.json,.csv,.ts,.js,.tsx,.jsx,.log,.yaml,.yml,.toml,.env,.html,.css,.xml,.py,.rb,.go,.rs,.java,.c,.h,.cpp,.sql"

export function AttachButton({ disabled }: { disabled?: boolean }) {
	const attachments = usePromptInputAttachments()
	return (
		<PromptInputButton
			tooltip="Attach files"
			onClick={() => attachments.openFileDialog()}
			disabled={disabled}
		>
			<PlusIcon className="size-4" />
		</PromptInputButton>
	)
}
