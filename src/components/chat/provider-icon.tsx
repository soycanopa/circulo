import { Bot, Brain, Sparkles, Zap } from "lucide-react"
import { cn } from "@/lib/utils"
import { formatProviderLabel } from "@/lib/model-groups"

const PROVIDER_STYLES: Record<string, { className: string; Icon: typeof Bot }> = {
	opencode: { className: "bg-sky-500/20 text-sky-300", Icon: Zap },
	minimax: { className: "bg-violet-500/20 text-violet-300", Icon: Sparkles },
	anthropic: { className: "bg-orange-500/20 text-orange-300", Icon: Brain },
	openai: { className: "bg-emerald-500/20 text-emerald-300", Icon: Bot },
	google: { className: "bg-blue-500/20 text-blue-300", Icon: Sparkles },
}

interface ProviderIconProps {
	providerId: string
	className?: string
	size?: "xs" | "sm"
}

export function ProviderIcon({ providerId, className, size = "xs" }: ProviderIconProps) {
	const key = providerId.trim().toLowerCase()
	const style = PROVIDER_STYLES[key]
	const Icon = style?.Icon ?? Bot
	const label = formatProviderLabel(providerId)

	return (
		<span
			title={label}
			className={cn(
				"inline-flex shrink-0 items-center justify-center rounded",
				size === "xs" ? "size-4" : "size-5",
				style?.className ?? "bg-muted text-muted-foreground",
				className,
			)}
		>
			<Icon className={size === "xs" ? "size-2.5" : "size-3"} />
		</span>
	)
}