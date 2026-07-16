import type { SessionInfo } from "@/types/acp"

export const HEATMAP_WINDOW_DAYS = 274

export interface ProfileHeatmapCell {
	day: string
	count: number
	weekday: number
	intensity: number
}

interface ProfileActivityStore {
	promptsByDay: Record<string, number>
	tokensByDay: Record<string, number>
	lifetimeTokens: number
	peakDayTokens: number
}

const STORAGE_KEY = "circulo-profile-activity"

function readStore(): ProfileActivityStore {
	try {
		const raw = localStorage.getItem(STORAGE_KEY)
		if (!raw) {
			return {
				promptsByDay: {},
				tokensByDay: {},
				lifetimeTokens: 0,
				peakDayTokens: 0,
			}
		}
		const parsed: unknown = JSON.parse(raw)
		if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
			return {
				promptsByDay: {},
				tokensByDay: {},
				lifetimeTokens: 0,
				peakDayTokens: 0,
			}
		}
		const data = parsed as Partial<ProfileActivityStore>
		return {
			promptsByDay:
				data.promptsByDay && typeof data.promptsByDay === "object"
					? data.promptsByDay
					: {},
			tokensByDay:
				data.tokensByDay && typeof data.tokensByDay === "object" ? data.tokensByDay : {},
			lifetimeTokens:
				typeof data.lifetimeTokens === "number" && Number.isFinite(data.lifetimeTokens)
					? data.lifetimeTokens
					: 0,
			peakDayTokens:
				typeof data.peakDayTokens === "number" && Number.isFinite(data.peakDayTokens)
					? data.peakDayTokens
					: 0,
		}
	} catch {
		return {
			promptsByDay: {},
			tokensByDay: {},
			lifetimeTokens: 0,
			peakDayTokens: 0,
		}
	}
}

function writeStore(store: ProfileActivityStore) {
	localStorage.setItem(STORAGE_KEY, JSON.stringify(store))
}

export function getLocalDayKey(date = new Date()): string {
	const year = date.getFullYear()
	const month = String(date.getMonth() + 1).padStart(2, "0")
	const day = String(date.getDate()).padStart(2, "0")
	return `${year}-${month}-${day}`
}

function weekdayOf(dayKey: string): number {
	const [year, month, day] = dayKey.split("-").map(Number)
	return new Date(year!, month! - 1, day!).getDay()
}

function addDaysIso(dayKey: string, offset: number): string {
	const [year, month, day] = dayKey.split("-").map(Number)
	const date = new Date(year!, month! - 1, day!)
	date.setDate(date.getDate() + offset)
	return getLocalDayKey(date)
}

function heatmapIntensity(count: number, max: number): number {
	if (count <= 0 || max <= 0) return 0
	const ratio = count / max
	if (ratio <= 0.25) return 1
	if (ratio <= 0.5) return 2
	if (ratio <= 0.75) return 3
	return 4
}

function buildHeatmap(countByDay: ReadonlyMap<string, number>, todayKey: string): ProfileHeatmapCell[] {
	const windowStart = addDaysIso(todayKey, -(HEATMAP_WINDOW_DAYS - 1))
	let windowMax = 0

	for (let offset = 0; offset < HEATMAP_WINDOW_DAYS; offset += 1) {
		const day = addDaysIso(windowStart, offset)
		windowMax = Math.max(windowMax, countByDay.get(day) ?? 0)
	}

	const heatmap: ProfileHeatmapCell[] = []
	for (let offset = 0; offset < HEATMAP_WINDOW_DAYS; offset += 1) {
		const day = addDaysIso(windowStart, offset)
		const count = countByDay.get(day) ?? 0
		heatmap.push({
			day,
			count,
			weekday: weekdayOf(day),
			intensity: heatmapIntensity(count, windowMax),
		})
	}
	return heatmap
}

function notifyProfileActivityChanged() {
	if (typeof window !== "undefined") {
		window.dispatchEvent(new CustomEvent("circulo:profile-activity-changed"))
	}
}

export function recordProfilePrompt(amount = 1) {
	if (amount <= 0) return
	const store = readStore()
	const day = getLocalDayKey()
	store.promptsByDay[day] = (store.promptsByDay[day] ?? 0) + amount
	writeStore(store)
	notifyProfileActivityChanged()
}

export function recordProfileTokens(tokens: number) {
	if (!Number.isFinite(tokens) || tokens <= 0) return
	const store = readStore()
	const day = getLocalDayKey()
	store.tokensByDay[day] = (store.tokensByDay[day] ?? 0) + tokens
	store.lifetimeTokens += tokens
	const dayTotal = store.tokensByDay[day] ?? 0
	store.peakDayTokens = Math.max(store.peakDayTokens, dayTotal)
	writeStore(store)
	notifyProfileActivityChanged()
}

export function seedProfileActivityFromSessions(sessions: SessionInfo[]) {
	const store = readStore()
	const seededCounts = new Map<string, number>()

	for (const session of sessions) {
		if (!session.updatedAt) continue
		const parsed = Date.parse(session.updatedAt)
		if (Number.isNaN(parsed)) continue
		const day = getLocalDayKey(new Date(parsed))
		seededCounts.set(day, (seededCounts.get(day) ?? 0) + 1)
	}

	let changed = false
	for (const [day, count] of seededCounts) {
		const existing = store.promptsByDay[day] ?? 0
		if (existing >= count) continue
		store.promptsByDay[day] = count
		changed = true
	}

	if (changed) {
		writeStore(store)
		notifyProfileActivityChanged()
	}
}

export function getProfileActivitySummary() {
	const store = readStore()
	const totalPrompts = Object.values(store.promptsByDay).reduce((sum, count) => sum + count, 0)
	const { currentStreakDays, longestStreakDays } = computeStreaks(store.promptsByDay)
	const tokenDays = Object.keys(store.tokensByDay).length

	return {
		totalPrompts,
		currentStreakDays,
		longestStreakDays,
		lifetimeTokens: store.lifetimeTokens,
		peakDayTokens: store.peakDayTokens,
		hasTokenHeatmap: tokenDays > 0,
	}
}

export function buildProfileHeatmap(
	metric: "prompts" | "tokens" = "prompts",
): ProfileHeatmapCell[] {
	const store = readStore()
	const source = metric === "tokens" ? store.tokensByDay : store.promptsByDay
	const countByDay = new Map(Object.entries(source))
	return buildHeatmap(countByDay, getLocalDayKey())
}

export function selectProfileHeatmap(): {
	cells: ProfileHeatmapCell[]
	unit: "prompts" | "tokens"
} {
	const summary = getProfileActivitySummary()
	if (summary.hasTokenHeatmap) {
		return { cells: buildProfileHeatmap("tokens"), unit: "tokens" }
	}
	return { cells: buildProfileHeatmap("prompts"), unit: "prompts" }
}

function computeStreaks(countByDay: Record<string, number>): {
	currentStreakDays: number
	longestStreakDays: number
} {
	const activeDays = new Set(
		Object.entries(countByDay)
			.filter(([, count]) => count > 0)
			.map(([day]) => day),
	)

	if (activeDays.size === 0) {
		return { currentStreakDays: 0, longestStreakDays: 0 }
	}

	let longest = 0
	let run = 0
	const today = getLocalDayKey()

	for (let offset = -(HEATMAP_WINDOW_DAYS - 1); offset <= 0; offset += 1) {
		const day = addDaysIso(today, offset)
		if (activeDays.has(day)) {
			run += 1
			longest = Math.max(longest, run)
		} else {
			run = 0
		}
	}

	let current = 0
	for (let offset = 0; offset >= -(HEATMAP_WINDOW_DAYS - 1); offset -= 1) {
		const day = addDaysIso(today, offset)
		if (!activeDays.has(day)) break
		current += 1
	}

	return { currentStreakDays: current, longestStreakDays: longest }
}

export function formatCompact(value: number | null | undefined): string {
	if (value == null || !Number.isFinite(value)) return "—"
	if (value < 1_000) return `${Math.round(value)}`
	if (value < 10_000) return `${(value / 1_000).toFixed(1).replace(/\.0$/, "")}K`
	if (value < 1_000_000) return `${Math.round(value / 1_000)}K`
	return `${(value / 1_000_000).toFixed(1).replace(/\.0$/, "")}M`
}

export function formatDays(value: number): string {
	if (value <= 0) return "0 days"
	return value === 1 ? "1 day" : `${value} days`
}

export function formatShortDate(dayKey: string): string | null {
	const [year, month, day] = dayKey.split("-").map(Number)
	if (!year || !month || !day) return null
	return new Date(year, month - 1, day).toLocaleDateString(undefined, {
		month: "short",
		day: "numeric",
		year: "numeric",
	})
}