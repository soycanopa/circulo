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
import { Loader2Icon, MonitorIcon, MoonIcon, SunIcon } from "lucide-react"
import { useCallback, useEffect, useRef, useState } from "react"
import { type DisplayMode, displayModeAtom, opaqueWindowsAtom, rtkEnabledAtom } from "../../atoms/preferences"
import { useColorScheme, useSetColorScheme } from "../../hooks/use-theme"
import type { ColorScheme } from "../../lib/themes"
import { checkRtk, disableRtk, enableRtk, fetchOpenInTargets, installRtk, setOpenInPreferred } from "../../services/backend"
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
				<RtkRow />
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

interface RtkState {
	checking: boolean
	installed: boolean
	version: string | null
	pluginActive: boolean
	enabling: boolean
	error: string | null
}

function RtkRow() {
	const enabled = useAtomValue(rtkEnabledAtom)
	const setEnabled = useSetAtom(rtkEnabledAtom)
	const [state, setState] = useState<RtkState>({
		checking: true,
		installed: false,
		version: null,
		pluginActive: false,
		enabling: false,
		error: null,
	})
	const pollRef = useRef<ReturnType<typeof setInterval> | null>(null)
	const pollTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

	useEffect(() => {
		if (!isElectron) {
			setState((s) => ({ ...s, checking: false }))
			return
		}
		checkRtk().then((result) => {
			setState({
				checking: false,
				installed: result.installed,
				version: result.version,
				pluginActive: result.pluginActive,
				enabling: false,
				error: null,
			})
		})
	}, [])

	useEffect(() => {
		if (!isElectron) return
		return window.circulo.onSettingsChanged((settings) => {
			const rtkSetting = (settings as { rtkEnabled?: boolean }).rtkEnabled
			if (typeof rtkSetting === "boolean" && rtkSetting !== enabled) {
				setEnabled(rtkSetting)
			}
		})
	}, [enabled, setEnabled])

	useEffect(() => {
		return () => {
			if (pollRef.current) {
				clearInterval(pollRef.current)
			}
			if (pollTimeoutRef.current) {
				clearTimeout(pollTimeoutRef.current)
			}
		}
	}, [])

	const handleToggle = useCallback(
		async (checked: boolean) => {
			setState((s) => ({ ...s, enabling: true, error: null }))
			try {
				if (checked) {
					const result = await enableRtk()
					if (result.success) {
						setEnabled(true)
						setState((s) => ({ ...s, enabling: false, pluginActive: true }))
					} else {
						setState((s) => ({
							...s,
							enabling: false,
							error: result.error ?? "Failed to enable RTK",
						}))
					}
				} else {
					const result = await disableRtk()
					if (result.success) {
						setEnabled(false)
						setState((s) => ({ ...s, enabling: false, pluginActive: false }))
					} else {
						setState((s) => ({ ...s, enabling: false, error: "Failed to disable RTK" }))
					}
				}
			} catch (err) {
				setState((s) => ({
					...s,
					enabling: false,
					error: err instanceof Error ? err.message : "Unknown error",
				}))
			}
		},
		[setEnabled],
	)

	const handleInstall = useCallback(async () => {
		await installRtk()
		pollRef.current = setInterval(async () => {
			const result = await checkRtk()
			if (result.installed) {
				if (pollRef.current) {
					clearInterval(pollRef.current)
					pollRef.current = null
				}
				if (pollTimeoutRef.current) {
					clearTimeout(pollTimeoutRef.current)
					pollTimeoutRef.current = null
				}
				setState({
					checking: false,
					installed: true,
					version: result.version,
					pluginActive: result.pluginActive,
					enabling: false,
					error: null,
				})
			}
		}, 2000)
		pollTimeoutRef.current = setTimeout(() => {
			if (pollRef.current) {
				clearInterval(pollRef.current)
				pollRef.current = null
			}
			pollTimeoutRef.current = null
			setState((s) => ({ ...s, checking: false, error: "Installation timeout" }))
		}, 60_000)
	}, [])

	if (!isElectron) return null

	const isLoading = state.checking || state.enabling

	return (
		<SettingsRow
			label="RTK Token Optimization"
			description="Reduce LLM token consumption by 60-90% by filtering command outputs before they reach the model. Requires the rtk CLI tool."
		>
			<div className="flex items-center gap-3">
				<Switch
					checked={enabled}
					onCheckedChange={handleToggle}
					disabled={!state.installed || isLoading}
				/>
				{isLoading && (
					<span className="text-xs text-muted-foreground">
						<Loader2Icon aria-hidden="true" className="inline size-3 animate-spin mr-1" />
						{state.enabling ? "Restarting server..." : "Checking..."}
					</span>
				)}
				{!isLoading && state.installed && (
					<span className="text-xs text-muted-foreground">
						{enabled
							? `Active — rtk v${state.version ?? "?"}`
							: `rtk v${state.version ?? "?"} detected`}
					</span>
				)}
				{!isLoading && !state.installed && (
					<button
						type="button"
						onClick={handleInstall}
						className="inline-flex items-center gap-1 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
					>
						Install RTK
					</button>
				)}
				{!isLoading && !state.installed && enabled && (
					<span className="text-xs text-amber-500">
						RTK not found — toggle disabled until reinstalled
					</span>
				)}
				{state.error && (
					<span className="text-xs text-destructive">{state.error}</span>
				)}
			</div>
		</SettingsRow>
	)
}
