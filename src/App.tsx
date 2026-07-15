import { Provider } from "jotai"
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
		</Provider>
	)
}