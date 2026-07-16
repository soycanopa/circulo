import { KeyRound, ShieldCheck, ShieldOff, XCircle } from "lucide-react"
import { credentialModeLabel } from "@/lib/credential-presentation"
import { cn } from "@/lib/utils"
import type { ChatMessage } from "@/types/acp"

interface AuthRequestCardProps {
	message: ChatMessage
}

const STATUS_COPY = {
	provided: "Credenciales proporcionadas",
	declined: "Credenciales rechazadas",
	cancelled: "Solicitud cancelada",
} as const

export function AuthRequestCard({ message }: AuthRequestCardProps) {
	const meta = message.authMeta
	if (!meta) return null

	const StatusIcon =
		meta.status === "provided"
			? ShieldCheck
			: meta.status === "declined"
				? ShieldOff
				: XCircle

	return (
		<div
			className={cn(
				"overflow-hidden rounded-xl border bg-card/60 shadow-sm",
				meta.status === "provided" && "border-diff-addition/30",
				meta.status === "declined" && "border-destructive/30",
				meta.status === "cancelled" && "border-border",
			)}
		>
			<div className="flex items-center gap-2 border-b border-border/50 px-4 py-2.5">
				<KeyRound className="size-3.5 text-[#3B5EF9]" />
				<span className="text-sm font-medium text-foreground">{meta.title}</span>
				<span className="rounded-md border border-border px-1.5 py-0.5 text-[10px] text-muted-foreground">
					{credentialModeLabel(meta.mode)}
				</span>
			</div>
			<div className="flex items-start gap-2 px-4 py-3">
				<StatusIcon
					className={cn(
						"mt-0.5 size-4 shrink-0",
						meta.status === "provided" && "text-diff-addition",
						meta.status === "declined" && "text-destructive",
						meta.status === "cancelled" && "text-muted-foreground",
					)}
				/>
				<div className="min-w-0">
					<p className="text-xs font-medium text-foreground">{STATUS_COPY[meta.status]}</p>
					<pre className="mt-1 whitespace-pre-wrap font-mono text-[11px] leading-relaxed text-muted-foreground">
						{message.content}
					</pre>
				</div>
			</div>
		</div>
	)
}