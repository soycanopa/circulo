import { describe, expect, it } from "vitest"
import {
	filterSlashCommands,
	getActiveSlash,
} from "@/lib/slash-parser"
import { DEFAULT_SLASH_COMMANDS } from "@/lib/slash-commands"

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
		expect(filterSlashCommands("", DEFAULT_SLASH_COMMANDS)).toHaveLength(3)
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
