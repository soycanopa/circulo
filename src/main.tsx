import React from "react"
import ReactDOM from "react-dom/client"
import "katex/dist/katex.min.css"
import App from "./App"
import "./styles/globals.css"

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<div className="h-full min-h-0 overflow-hidden bg-transparent">
			<App />
		</div>
	</React.StrictMode>,
)
