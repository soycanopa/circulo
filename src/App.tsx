import { Provider } from "jotai"
import { AppShell } from "@/components/layout/app-shell"

export function App() {
	return (
		<Provider>
			<AppShell />
		</Provider>
	)
}