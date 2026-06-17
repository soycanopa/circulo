import { Input } from "@circulo/ui/components/input"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@circulo/ui/components/select"
import { Switch } from "@circulo/ui/components/switch"
import { useAtomValue, useSetAtom } from "jotai"
import { useCallback, useEffect, useState } from "react"
import { type DisplayMode, displayModeAtom, opaqueWindowsAtom } from "../../atoms/preferences"
import { useColorScheme, useSetColorScheme } from "../../hooks/use-theme"
import type { ColorScheme } from "../../lib/themes"
import { fetchOpenInTargets, setOpenInPreferred } from "../../services/backend"
import { SettingsRow } from "./settings-row"
import { SettingsSection } from "./settings-section"

const isElectron = typeof window !== "undefined" && "circulo" in window
const isMac = isElectron && window.circulo.platform === "darwin"

export function GeneralSettings() {
	return (
		<div className="space-y-8">
			<div>
				<h2 className="text-xl font-semibold">General</h2>
			</div>

			<SettingsSection>
				<OpenDestinationRow />
			</SettingsSection>

			<SettingsSection title="Appearance">
				<ThemeRow />
				{isMac && <OpaqueWindowsRow />}
				<OpencodeBinaryRow />
				<DisplayModeRow />
			</SettingsSection>
		</div>
	)
}

function OpenDestinationRow() {
	const [targets, setTargets] = useState<{ id: string; label: string; available: boolean }[]>([])
	const [preferred, setPreferred] = useState<string | null>(null)

	useEffect(() => {
		if (!isElectron) return
		fetchOpenInTargets().then((result) => {
			setTargets(result.targets.filter((t) => t.available))
			setPreferred(result.preferredTarget)
		})
	}, [])

	const handleChange = useCallback(async (value: string) => {
		setPreferred(value)
		await setOpenInPreferred(value)
	}, [])

	if (targets.length === 0) return null

	return (
		<SettingsRow
			label="Default open destination"
			description="Where files and folders open by default"
		>
			<Select
				value={preferred ?? undefined}
				onValueChange={(v) => {
					if (v !== null) handleChange(v)
				}}
			>
				<SelectTrigger className="min-w-[180px]">
					<SelectValue placeholder="Select..." />
				</SelectTrigger>
				<SelectContent>
					{targets.map((t) => (
						<SelectItem key={t.id} value={t.id}>
							{t.label}
						</SelectItem>
					))}
				</SelectContent>
			</Select>
		</SettingsRow>
	)
}

function ThemeRow() {
	const colorScheme = useColorScheme()
	const setColorScheme = useSetColorScheme()

	const options: { value: ColorScheme; label: string; icon: typeof SunIcon }[] = [
		{ value: "light", label: "Light", icon: SunIcon },
		{ value: "dark", label: "Dark", icon: MoonIcon },
		{ value: "system", label: "System", icon: MonitorIcon },
	]

	return (
		<SettingsRow label="Theme" description="Use light, dark, or match your system">
			<div className="flex items-center rounded-md border border-border">
				{options.map((opt) => {
					const Icon = opt.icon
					const isActive = colorScheme === opt.value
					return (
						<button
							key={opt.value}
							type="button"
							onClick={() => setColorScheme(opt.value)}
							className={`flex items-center gap-1.5 px-3 py-1.5 text-sm transition-colors first:rounded-l-md last:rounded-r-md ${
								isActive
									? "bg-accent text-accent-foreground font-medium"
									: "text-muted-foreground hover:text-foreground"
							}`}
						>
							<Icon aria-hidden="true" className="size-3.5" />
							{opt.label}
						</button>
					)
				})}
			</div>
		</SettingsRow>
	)
}

function OpaqueWindowsRow() {
	const opaque = useAtomValue(opaqueWindowsAtom)
	const setOpaque = useSetAtom(opaqueWindowsAtom)

	const handleChange = useCallback(
		async (checked: boolean) => {
			setOpaque(checked)
			if (isElectron) {
				await window.circulo.setOpaqueWindows(checked)
				// Requires relaunch -- prompt or auto-relaunch
				window.circulo.relaunch()
			}
		},
		[setOpaque],
	)

	return (
		<SettingsRow
			label="Use opaque background"
			description="Make windows use a solid background rather than system translucency"
		>
			<Switch checked={opaque} onCheckedChange={handleChange} />
		</SettingsRow>
	)
}

function DisplayModeRow() {
	const displayMode = useAtomValue(displayModeAtom)
	const setDisplayMode = useSetAtom(displayModeAtom)

	return (
		<SettingsRow
			label="Display mode"
			description="Adjust how much detail is shown in conversations"
		>
			<Select
				value={displayMode}
				onValueChange={(v) => setDisplayMode(v as DisplayMode)}
				items={{ default: "Default", verbose: "Verbose" }}
			>
				<SelectTrigger className="min-w-[140px]">
					<SelectValue />
				</SelectTrigger>
				<SelectContent>
					<SelectItem value="default">Default</SelectItem>
					<SelectItem value="verbose">Verbose</SelectItem>
				</SelectContent>
			</Select>
		</SettingsRow>
	)
}

function OpencodeBinaryRow() {
	const [binary, setBinary] = useState("")
	const [loaded, setLoaded] = useState(false)

	useEffect(() => {
		if (!isElectron) return
		window.circulo.getOpencodeBinary().then((val) => {
			setBinary(val ?? "")
			setLoaded(true)
		})
	}, [])

	const handleChange = useCallback(
		async (value: string) => {
			setBinary(value)
			if (!isElectron) return
			const trimmed = value.trim()
			await window.circulo.setOpencodeBinary(trimmed || null)
		},
		[],
	)

	if (!isElectron || !loaded) return null

	return (
		<SettingsRow
			label="OpenCode binary"
			description="Custom path or name for the opencode CLI binary. Leave empty to use auto-detection (opencode or opencode-cli on PATH)."
		>
			<Input
				className="w-[260px]"
				placeholder="opencode"
				value={binary}
				onChange={(e) => handleChange(e.target.value)}
			/>
		</SettingsRow>
	)
}
