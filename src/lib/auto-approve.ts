import type { ConfigOption, PermissionOption } from "@/types/acp"

const AUTO_APPROVE_CONFIG =
	/\b(brave|always.?approve|auto.?approve|auto.?permission|yolo)\b/

const AUTO_APPROVE_VALUE =
	/\b(true|on|yes|allow|always|auto|yolo|full|unrestricted|enabled)\b/

const AUTO_APPROVE_OFF_VALUE =
	/\b(false|off|no|ask|deny|restricted|disabled|manual)\b/

export function findAutoApproveConfigOption(
	options: ConfigOption[],
): ConfigOption | null {
	for (const option of options) {
		const haystack = [option.id, option.name, option.category ?? ""]
			.join(" ")
			.toLowerCase()
		if (AUTO_APPROVE_CONFIG.test(haystack)) {
			return option
		}
	}
	return null
}

export function isAutoApproveConfigEnabled(option: ConfigOption): boolean {
	const value = option.currentValue.trim().toLowerCase()
	if (!value) return false
	if (AUTO_APPROVE_OFF_VALUE.test(value)) return false
	if (AUTO_APPROVE_VALUE.test(value)) return true

	const label =
		option.options
			.find((item) => item.value === option.currentValue)
			?.name.toLowerCase() ?? ""
	return AUTO_APPROVE_VALUE.test(label)
}

export function autoApproveConfigValue(
	option: ConfigOption,
	enabled: boolean,
): string | null {
	if (option.options.length === 0) {
		return enabled ? "true" : "false"
	}

	const match = (pattern: RegExp) =>
		option.options.find(
			(item) =>
				pattern.test(item.value.toLowerCase()) ||
				pattern.test(item.name.toLowerCase()),
		)?.value ?? null

	if (enabled) {
		return (
			match(/\ballow.?always\b|\balways.?allow\b/) ??
			match(/\bauto\b|\byolo\b|\bfull\b|\bunrestricted\b/) ??
			match(/\ballow\b|\bapprove\b|\bon\b|\btrue\b/) ??
			option.options[option.options.length - 1]?.value ??
			null
		)
	}

	return (
		match(/\bask\b|\bmanual\b|\boff\b|\bfalse\b|\bdeny\b|\brestricted\b/) ??
		match(/\bonce\b/) ??
		option.options[0]?.value ??
		null
	)
}

export function pickAutoApprovePermissionOption(
	options: PermissionOption[],
): string | null {
	for (const option of options) {
		const id = option.optionId.toLowerCase()
		const name = option.name.toLowerCase()
		if (
			/allow.?always|always.?allow|allow_always|allow-always/.test(id) ||
			name.includes("always")
		) {
			return option.optionId
		}
	}

	for (const option of options) {
		const id = option.optionId.toLowerCase()
		if (/allow|approve|accept/.test(id) && !/deny|reject|once/.test(id)) {
			return option.optionId
		}
	}

	return options[0]?.optionId ?? null
}
