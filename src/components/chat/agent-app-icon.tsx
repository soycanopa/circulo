import { Bot } from "lucide-react"
import type { ComponentType, SVGProps } from "react"
import { OpencodeBrandIcon, XaiBrandIcon } from "@/components/chat/provider-brand-icons"
import type { AgentProviderId } from "@/lib/agent-providers"
import { cn } from "@/lib/utils"

const AGENT_ICONS: Partial<Record<AgentProviderId, ComponentType<SVGProps<SVGSVGElement>>>> = {
	opencode: OpencodeBrandIcon,
	grok: XaiBrandIcon,
}

interface AgentAppIconProps {
	agentId: AgentProviderId
	className?: string
}

export function AgentAppIcon({ agentId, className }: AgentAppIconProps) {
	const Icon = AGENT_ICONS[agentId] ?? Bot
	return <Icon className={cn("size-3.5 shrink-0", className)} aria-hidden />
}