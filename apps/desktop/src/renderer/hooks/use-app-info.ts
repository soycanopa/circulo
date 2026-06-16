import { useEffect, useState } from "react"
import type { AppInfo } from "../../preload/api"

const isElectron = typeof window !== "undefined" && "circulo" in window

export function useAppInfo() {
	const [info, setInfo] = useState<AppInfo | null>(null)

	useEffect(() => {
		if (!isElectron) return
		window.circulo.getAppInfo().then(setInfo)
	}, [])

	return info
}
