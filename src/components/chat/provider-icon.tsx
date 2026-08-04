import { Sparkles } from "lucide-react"
import type { ReactNode } from "react"
import { normalizeProviderId } from "@/lib/provider-registry"
import { cn } from "@/lib/utils"

interface ProviderIconProps {
	providerId: string
	className?: string
}

const MONO = "text-white/55"

function IconSvg({
	children,
	className,
}: {
	children: ReactNode
	className?: string
}) {
	return (
		<svg
			viewBox="0 0 16 16"
			className={cn("size-3.5 shrink-0", MONO, className)}
			fill="currentColor"
			aria-hidden
		>
			{children}
		</svg>
	)
}

export function ProviderIcon({ providerId, className }: ProviderIconProps) {
	const id = normalizeProviderId(providerId)

	switch (id) {
		case "opencode":
			return (
				<IconSvg className={className}>
					<circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" strokeWidth="1.2" />
					<path
						d="M5.5 8h5M8 5.5v5"
						fill="none"
						stroke="currentColor"
						strokeWidth="1.2"
						strokeLinecap="round"
					/>
				</IconSvg>
			)
		case "minimax":
			return (
				<IconSvg className={className}>
					<rect
						x="3"
						y="3"
						width="10"
						height="10"
						rx="2"
						fill="none"
						stroke="currentColor"
						strokeWidth="1.2"
					/>
					<path
						d="M5.5 11V5l2.5 2.5L10.5 5v6"
						fill="none"
						stroke="currentColor"
						strokeWidth="1.1"
						strokeLinejoin="round"
					/>
				</IconSvg>
			)
		case "anthropic":
			return (
				<IconSvg className={className}>
					<path d="M8 4.5 10.8 11.5H5.2L8 4.5Z" fill="currentColor" opacity="0.9" />
				</IconSvg>
			)
		case "openai":
			return (
				<IconSvg className={className}>
					<circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" strokeWidth="1.2" />
					<circle cx="8" cy="8" r="2.2" fill="currentColor" />
				</IconSvg>
			)
		case "google":
			return (
				<IconSvg className={className}>
					<path
						d="M8 2.5a5.5 5.5 0 0 1 4.8 8.2H8v-2.4h4.5A5.5 5.5 0 1 0 8 13.5v-2.4A3.1 3.1 0 1 1 11.1 8H8V5.6A5.5 5.5 0 0 1 8 2.5Z"
						fill="currentColor"
						opacity="0.85"
					/>
				</IconSvg>
			)
		case "grok":
			return (
				<IconSvg className={className}>
					<circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" strokeWidth="1.2" />
					<path
						d="M5.5 8h5M8 5.5l2.5 2.5L8 10.5 5.5 8"
						fill="none"
						stroke="currentColor"
						strokeWidth="1.1"
						strokeLinecap="round"
						strokeLinejoin="round"
					/>
				</IconSvg>
			)
		default:
			return (
				<Sparkles
					className={cn("size-3.5 shrink-0 text-white/55", className)}
				/>
			)
	}
}
