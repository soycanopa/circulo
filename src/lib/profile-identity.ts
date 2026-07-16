import { useCallback, useEffect, useMemo, useState } from "react"

const NAME_KEY = "circulo:profile:name:v1"
const HANDLE_KEY = "circulo:profile:handle:v1"
const AVATAR_COLOR_KEY = "circulo:profile:avatar-color:v1"
const AVATAR_IMAGE_KEY = "circulo:profile:avatar-image:v1"

export const PROFILE_CHANGED_EVENT = "circulo:profile-changed"

export function notifyProfileChanged() {
	window.dispatchEvent(new CustomEvent(PROFILE_CHANGED_EVENT))
}

export function isProfileConfigured(): boolean {
	return Boolean(
		readStorage(NAME_KEY) ||
			readStorage(HANDLE_KEY) ||
			readStorage(AVATAR_IMAGE_KEY) ||
			readStorage(AVATAR_COLOR_KEY),
	)
}

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
		notifyProfileChanged()
	}, [])

	return { name, setName }
}

export function useProfileHandle(defaultHandle: string) {
	const [handle, setHandleState] = useState(() => readStorage(HANDLE_KEY) ?? defaultHandle)

	const setHandle = useCallback((value: string) => {
		const trimmed = value.trim()
		if (!trimmed) return
		const next = trimmed.startsWith("@") ? trimmed : `@${trimmed}`
		setHandleState(next)
		writeStorage(HANDLE_KEY, next)
		notifyProfileChanged()
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
		notifyProfileChanged()
	}, [])

	return { color, setColor }
}

export function useProfileAvatarImage() {
	const [stored, setStoredState] = useState(() => readStorage(AVATAR_IMAGE_KEY) ?? "")
	const image = stored.trim().length > 0 ? stored : null

	const setImage = useCallback((value: string | null) => {
		const next = value ?? ""
		setStoredState(next)
		if (next) {
			writeStorage(AVATAR_IMAGE_KEY, next)
		} else {
			localStorage.removeItem(AVATAR_IMAGE_KEY)
		}
		notifyProfileChanged()
	}, [])

	return { image, setImage }
}

/** Sidebar + shell: live profile snapshot with configured flag. */
export function useProfileIdentity() {
	const defaultName = getDefaultProfileName()
	const defaultHandle = getDefaultProfileHandle()
	const { name } = useProfileName(defaultName)
	const { handle } = useProfileHandle(defaultHandle)
	const { color } = useProfileAvatarColor()
	const { image } = useProfileAvatarImage()
	const [revision, setRevision] = useState(0)

	useEffect(() => {
		function refresh() {
			setRevision((value) => value + 1)
		}
		window.addEventListener(PROFILE_CHANGED_EVENT, refresh)
		return () => window.removeEventListener(PROFILE_CHANGED_EVENT, refresh)
	}, [])

	const configured = useMemo(() => isProfileConfigured(), [revision, name, handle, image])

	return {
		name,
		handle,
		color,
		image,
		initials: getProfileInitials(name),
		configured,
	}
}