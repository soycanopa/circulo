import { describe, expect, it } from "vitest"
import {
	autoApproveConfigValue,
	findAutoApproveConfigOption,
	isAutoApproveConfigEnabled,
	isAllowPermissionOption,
	matchesAllowedPattern,
	permissionToolName,
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

describe("matchesAllowedPattern", () => {
	it("matches exact tool names", () => {
		expect(matchesAllowedPattern("bash", ["bash"])).toBe(true)
		expect(matchesAllowedPattern("bash", ["edit"])).toBe(false)
	})

	it("matches simple globs", () => {
		expect(matchesAllowedPattern("edit", ["edit*"])).toBe(true)
		expect(matchesAllowedPattern("edit_file", ["edit*"])).toBe(true)
		expect(matchesAllowedPattern("read", ["edit*"])).toBe(false)
		expect(matchesAllowedPattern("anything", ["*"])).toBe(true)
	})

	it("matches case-insensitively", () => {
		expect(matchesAllowedPattern("Bash", ["bash"])).toBe(true)
	})

	it("ignores empty patterns", () => {
		expect(matchesAllowedPattern("bash", [""])).toBe(false)
	})
})

describe("isAllowPermissionOption", () => {
	it("flags approve options only", () => {
		expect(isAllowPermissionOption({ optionId: "allow-once", name: "Allow" })).toBe(
			true,
		)
		expect(isAllowPermissionOption({ optionId: "reject", name: "Reject" })).toBe(
			false,
		)
	})
})

describe("permissionToolName", () => {
	it("reads the tool name from the request payload", () => {
		expect(
			permissionToolName({ requestId: "r1", sessionId: "s1", toolCall: { title: "bash" }, options: [] }),
		).toBe("bash")
		expect(
			permissionToolName({ requestId: "r2", sessionId: "s1", toolCall: { name: "EditFile" }, options: [] }),
		).toBe("EditFile")
		expect(
			permissionToolName({ requestId: "r3", sessionId: "s1", options: [] }),
		).toBe("")
	})
})
