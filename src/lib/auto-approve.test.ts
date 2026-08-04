import { describe, expect, it } from "vitest"
import {
	autoApproveConfigValue,
	findAutoApproveConfigOption,
	isAutoApproveConfigEnabled,
	pickAutoApprovePermissionOption,
} from "@/lib/auto-approve"
import type { ConfigOption, PermissionOption } from "@/types/acp"

describe("auto-approve helpers", () => {
	it("finds brave mode config options", () => {
		const options: ConfigOption[] = [
			{
				id: "brave_mode",
				name: "Brave Mode",
				currentValue: "false",
				options: [
					{ value: "true", name: "On" },
					{ value: "false", name: "Off" },
				],
			},
		]
		expect(findAutoApproveConfigOption(options)?.id).toBe("brave_mode")
	})

	it("maps boolean config values", () => {
		const option: ConfigOption = {
			id: "brave_mode",
			name: "Brave Mode",
			currentValue: "false",
			options: [
				{ value: "true", name: "On" },
				{ value: "false", name: "Off" },
			],
		}
		expect(isAutoApproveConfigEnabled(option)).toBe(false)
		expect(autoApproveConfigValue(option, true)).toBe("true")
		expect(autoApproveConfigValue(option, false)).toBe("false")
	})

	it("prefers allow-always permission options", () => {
		const options: PermissionOption[] = [
			{ optionId: "allow-once", name: "Allow once" },
			{ optionId: "allow-always", name: "Allow always" },
			{ optionId: "reject-once", name: "Reject" },
		]
		expect(pickAutoApprovePermissionOption(options)).toBe("allow-always")
	})
})
