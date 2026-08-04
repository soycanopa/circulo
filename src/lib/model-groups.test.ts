import { describe, expect, it } from "vitest"
import { groupModelOptions, modelShortName } from "@/lib/model-groups"

describe("groupModelOptions", () => {
	it("groups by ACP group name and surfaces favorites first", () => {
		const options = [
			{
				value: "opencode/gpt-5.5",
				name: "GPT 5.5",
				group: "OpenCode Zen",
			},
			{
				value: "minimax/m2.5",
				name: "MiniMax M2.5",
				group: "MiniMax",
			},
			{
				value: "minimax/m3",
				name: "MiniMax M3",
				group: "MiniMax",
			},
		]

		const { favorites, groups } = groupModelOptions(options, [
			"minimax/m2.5",
		])

		expect(favorites).toHaveLength(1)
		expect(favorites[0]?.value).toBe("minimax/m2.5")
		expect(groups).toHaveLength(2)
		expect(groups.find((g) => g.providerLabel === "MiniMax")?.models).toHaveLength(
			1,
		)
	})

	it("groups ungrouped models by provider prefix", () => {
		const options = [
			{ value: "opencode/grok-code", name: "Grok Code" },
			{ value: "anthropic/claude-sonnet", name: "Claude Sonnet" },
		]

		const { groups } = groupModelOptions(options, [])
		expect(groups.map((g) => g.providerLabel).sort()).toEqual([
			"Anthropic",
			"OpenCode Zen",
		])
	})

	it("modelShortName returns only the model id suffix", () => {
		expect(
			modelShortName({
				value: "opencode/gpt-5.5",
				name: "OpenCode Zen / GPT 5.5",
			}),
		).toBe("GPT 5.5")
		expect(
			modelShortName({ value: "minimax/m2.5", name: "MiniMax M2.5" }),
		).toBe("MiniMax M2.5")
		expect(modelShortName({ value: "anthropic/claude-sonnet", name: "" })).toBe(
			"claude-sonnet",
		)
	})
})
