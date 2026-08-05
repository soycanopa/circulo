import { describe, expect, it } from "vitest"
import {
	filterSlashCommands,
	getActiveSlash,
} from "@/lib/slash-parser"
import {
	DEFAULT_SLASH_COMMANDS,
	mergeSlashCommands,
} from "@/lib/slash-commands"

describe("getActiveSlash", () => {
	it("detects a slash token at the start of the input", () => {
		expect(getActiveSlash("/comp", 5)).toEqual({ query: "comp", start: 0 })
	})

	it("returns empty query for a bare slash", () => {
		expect(getActiveSlash("/", 1)).toEqual({ query: "", start: 0 })
	})

	it("returns null when the slash is not at the start", () => {
		expect(getActiveSlash("hey /comp", 9)).toBeNull()
		expect(getActiveSlash("a/comp", 6)).toBeNull()
	})

	it("returns null once whitespace follows the command", () => {
		expect(getActiveSlash("/comp hey", 10)).toBeNull()
	})

	it("respects the cursor position", () => {
		expect(getActiveSlash("/compact", 4)).toEqual({
			query: "com",
			start: 0,
		})
	})
})

describe("filterSlashCommands", () => {
	it("returns all commands for an empty query", () => {
		expect(filterSlashCommands("", DEFAULT_SLASH_COMMANDS)).toHaveLength(4)
	})

	it("filters by prefix", () => {
		const results = filterSlashCommands("comp", DEFAULT_SLASH_COMMANDS)
		expect(results.map((c) => c.command)).toEqual(["compact"])
	})

	it("matches case-insensitively", () => {
		const results = filterSlashCommands("HELP", DEFAULT_SLASH_COMMANDS)
		expect(results.map((c) => c.command)).toEqual(["help"])
	})
})

describe("mergeSlashCommands", () => {
	it("appends custom commands and normalizes their token", () => {
		const merged = mergeSlashCommands([
			{ command: "/review", label: "Review the current diff", description: "Run a review" },
		])
		expect(merged).toHaveLength(5)
		const custom = merged[4]
		expect(custom.command).toBe("review")
		expect(custom.label).toBe("/review")
		expect(custom.prompt).toBe("Review the current diff")
		expect(custom.description).toBe("Run a review")
	})

	it("does not let custom commands shadow built-ins", () => {
		const merged = mergeSlashCommands([
			{ command: "/clear", label: "custom", description: "custom" },
			{ command: "/compact", label: "custom2", description: "custom2" },
		])
		expect(merged).toHaveLength(4)
		expect(merged.map((c) => c.command)).toEqual(["compact", "help", "clear", "mcp"])
		expect(merged[0].prompt).toBeUndefined()
	})

	it("returns built-ins when no custom commands exist", () => {
		expect(mergeSlashCommands()).toEqual(DEFAULT_SLASH_COMMANDS)
	})
})
