import { describe, expect, it } from "vitest"
import {
	extractMentionPaths,
	getActiveMention,
	insertMention,
} from "@/lib/mention-parser"

describe("extractMentionPaths", () => {
	it("collects unique @ paths from text", () => {
		expect(
			extractMentionPaths("See @src/App.tsx and @src/lib/utils.ts"),
		).toEqual(["src/App.tsx", "src/lib/utils.ts"])
	})
})

describe("getActiveMention", () => {
	it("returns query at cursor after @", () => {
		const text = "check @src/App"
		expect(getActiveMention(text, text.length)?.query).toBe("src/App")
	})

	it("ignores @ inside words", () => {
		expect(getActiveMention("email@test.com", 14)).toBeNull()
	})
})

describe("insertMention", () => {
	it("inserts path and places cursor after mention", () => {
		const { value, cursor } = insertMention("Hello @", 6, 7, "src/App.tsx")
		expect(value).toBe("Hello @src/App.tsx")
		expect(cursor).toBe("Hello @src/App.tsx".length)
	})
})
