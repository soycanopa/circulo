import { useAtom } from "jotai"
import { PlanPreviewCard } from "@/components/chat/plan-preview-card"
import { OverlayShell } from "@/components/overlays/overlay-shell"
import { planOverlayAtom } from "@/stores/atoms"

export function PlanPreviewOverlay() {
	const [planOverlay, setPlanOverlay] = useAtom(planOverlayAtom)
	if (!planOverlay) return null

	return (
		<OverlayShell
			open
			title="Plan propuesto"
			badge="Plan"
			onClose={() => setPlanOverlay(null)}
			className="max-w-4xl"
		>
			<div className="p-4">
				<PlanPreviewCard
					variant="standalone"
					showExpand={false}
					content={planOverlay.content}
					isStreaming={planOverlay.isStreaming}
					actionsEnabled={planOverlay.actionsEnabled}
					onDownload={() => planOverlay.onDownload?.()}
					onAccept={() => {
						planOverlay.onAccept?.()
						setPlanOverlay(null)
					}}
					onAcceptAndCompact={() => {
						planOverlay.onAcceptAndCompact?.()
						setPlanOverlay(null)
					}}
					onComment={() => {
						planOverlay.onComment?.()
						setPlanOverlay(null)
					}}
					onReject={() => {
						planOverlay.onReject?.()
						setPlanOverlay(null)
					}}
				/>
			</div>
		</OverlayShell>
	)
}