import { Provider } from "jotai"
import { PlanPreviewOverlay } from "@/components/overlays/plan-preview-overlay"
import { ToolPreviewOverlay } from "@/components/overlays/tool-preview-overlay"
import { AppShell } from "@/components/layout/app-shell"
import { useAcpEventBridge } from "@/hooks/use-acp-event-bridge"

function AcpBridge() {
	useAcpEventBridge()
	return null
}

export function App() {
	return (
		<Provider>
			<AcpBridge />
			<AppShell />
			<ToolPreviewOverlay />
			<PlanPreviewOverlay />
		</Provider>
	)
}