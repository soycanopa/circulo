import { useCallback, useState } from "react"

const NAME_KEY = "circulo:profile:name:v1"
const HANDLE_KEY = "circulo:profile:handle:v1"
const AVATAR_COLOR_KEY = "circulo:profile:avatar-color:v1"

export const PROFILE_AVATAR_COLORS = [
	"#6fcbf3",
	"#40c977",
	"#fa423e",
	"#f5a623",
	"#b794f6",
	"#ff6b9d",
] as const

function readStorage(key: string): string | null {
	try {
		return localStorage.getItem(key)
	} catch {
		return null
	}
}

function writeStorage(key: string, value: string) {
	localStorage.setItem(key, value)
}

export function getDefaultProfileName(): string {
	if (typeof navigator !== "undefined") {
		const platform = navigator.platform || ""
		if (/Mac|iPhone|iPad|iPod/.test(platform) || navigator.userAgent.includes("Mac OS X")) {
			return "Mac Developer"
		}
	}
	return "Circulo Developer"
}

export function getDefaultProfileHandle(): string {
	return "@circulo"
}

export function getProfileInitials(name: string): string {
	const parts = name.trim().split(/\s+/).filter(Boolean)
	if (parts.length === 0) return "C"
	if (parts.length === 1) return parts[0]!.slice(0, 2).toUpperCase()
	return `${parts[0]![0] ?? ""}${parts[1]![0] ?? ""}`.toUpperCase()
}

export function useProfileName(defaultName: string) {
	const [name, setNameState] = useState(() => readStorage(NAME_KEY) ?? defaultName)

	const setName = useCallback((value: string) => {
		const trimmed = value.trim()
		if (!trimmed) return
		setNameState(trimmed)
		writeStorage(NAME_KEY, trimmed)
	}, [])

	return { name, setName }
}

export function useProfileHandle(defaultHandle: string) {
	const [handle, setHandleState] = useState(() => readStorage(HANDLE_KEY) ?? defaultHandle)

	const setHandle = useCallback((value: string) => {
		const trimmed = value.trim()
		if (!trimmed) return
		setHandleState(trimmed.startsWith("@") ? trimmed : `@${trimmed}`)
		writeStorage(HANDLE_KEY, trimmed.startsWith("@") ? trimmed : `@${trimmed}`)
	}, [])

	return { handle, setHandle }
}

export function useProfileAvatarColor() {
	const [color, setColorState] = useState(
		() => readStorage(AVATAR_COLOR_KEY) ?? PROFILE_AVATAR_COLORS[0],
	)

	const setColor = useCallback((value: string) => {
		setColorState(value)
		writeStorage(AVATAR_COLOR_KEY, value)
	}, [])

	return { color, setColor }
}