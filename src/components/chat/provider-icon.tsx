import { Bot, Brain, Sparkles, Zap } from "lucide-react"
import type { ComponentType, SVGProps } from "react"
import {
	MinimaxBrandIcon,
	OpencodeBrandIcon,
	ZaiBrandIcon,
} from "@/components/chat/provider-brand-icons"
import { formatProviderLabel } from "@/lib/model-groups"
import { cn } from "@/lib/utils"

type ProviderSvg = ComponentType<SVGProps<SVGSVGElement>>

const PROVIDER_SVGS: Record<string, ProviderSvg> = {
	opencode: OpencodeBrandIcon,
	"opencode-go": OpencodeBrandIcon,
	minimax: MinimaxBrandIcon,
	"minimax-coding-plan": MinimaxBrandIcon,
	zai: ZaiBrandIcon,
	"zai-coding-plan": ZaiBrandIcon,
}

const PROVIDER_STYLES: Record<string, { className: string; Icon: typeof Bot }> = {
	anthropic: { className: "bg-orange-500/15 text-orange-300", Icon: Brain },
	openai: { className: "bg-emerald-500/15 text-emerald-300", Icon: Bot },
	google: { className: "bg-blue-500/15 text-blue-300", Icon: Sparkles },
	xai: { className: "bg-slate-500/15 text-slate-200", Icon: Zap },
}

const BRAND_ICON_STYLES: Record<string, string> = {
	opencode: "bg-white/8 text-foreground",
	"opencode-go": "bg-white/8 text-foreground",
	minimax: "bg-violet-500/15 text-violet-200",
	"minimax-coding-plan": "bg-violet-500/15 text-violet-200",
	zai: "bg-white/8 text-foreground",
	"zai-coding-plan": "bg-white/8 text-foreground",
}

function normalizeProviderId(providerId: string): string {
	return providerId.trim().toLowerCase()
}

interface ProviderIconProps {
	providerId: string
	className?: string
	size?: "xs" | "sm"
}

export function ProviderIcon({ providerId, className, size = "xs" }: ProviderIconProps) {
	const key = normalizeProviderId(providerId)
	const BrandIcon = PROVIDER_SVGS[key]
	const label = formatProviderLabel(providerId)
	const iconBox = size === "xs" ? "size-4" : "size-5"
	const iconSize = size === "xs" ? "size-2.5" : "size-3"

	if (BrandIcon) {
		return (
			<span
				title={label}
				className={cn(
					"inline-flex shrink-0 items-center justify-center rounded",
					iconBox,
					BRAND_ICON_STYLES[key] ?? "bg-white/8 text-foreground",
					className,
				)}
			>
				<BrandIcon className={cn(iconSize, "shrink-0")} />
			</span>
		)
	}

	const style = PROVIDER_STYLES[key]
	const Icon = style?.Icon ?? Bot

	return (
		<span
			title={label}
			className={cn(
				"inline-flex shrink-0 items-center justify-center rounded",
				iconBox,
				style?.className ?? "bg-muted text-muted-foreground",
				className,
			)}
		>
			<Icon className={iconSize} />
		</span>
	)
}