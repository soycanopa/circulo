import React from "react"
import ReactDOM from "react-dom/client"
import { App } from "@/App"
import { isTauri } from "@/lib/window-chrome"
import "@/styles/globals.css"

if (isTauri) {
	document.documentElement.dataset.tauri = ""
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<App />
	</React.StrictMode>,
)