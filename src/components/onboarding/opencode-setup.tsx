import { ExternalLink } from "lucide-react"
import type { OpencodeStatus } from "@/types/acp"

interface OpencodeSetupBannerProps {
	status: OpencodeStatus
}

export function OpencodeSetupBanner({ status }: OpencodeSetupBannerProps) {
	if (status.available) return null

	return (
		<div className="mx-4 mt-3 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-3 text-xs text-amber-100">
			<p className="font-medium">OpenCode not found</p>
			<p className="mt-1 text-amber-100/80">{status.installHint}</p>
			<ul className="mt-2 list-inside list-disc space-y-0.5 text-[11px] text-muted">
				<li>
					Install the CLI and ensure <code className="text-fg/90">opencode</code>{" "}
					is on your PATH
				</li>
				<li>
					Or set{" "}
					<code className="text-fg/90">OPENCODE_BIN</code> to the full path (e.g.{" "}
					<code className="text-fg/90">~/.opencode/bin/opencode</code>)
				</li>
			</ul>
			<a
				href="https://opencode.ai"
				target="_blank"
				rel="noreferrer"
				className="mt-2 inline-flex items-center gap-1 text-[11px] text-fg/90 underline-offset-2 hover:underline"
			>
				opencode.ai
				<ExternalLink className="size-3" />
			</a>
		</div>
	)
}
