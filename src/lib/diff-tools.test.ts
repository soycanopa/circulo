import { describe, expect, it } from "vitest"
import {
	collectDiffTools,
	collectSessionDiffs,
	isDiffTool,
	isGeneratedFile,
} from "@/lib/diff-tools"
import type { ChatMessage, ToolCall } from "@/types/acp"

function tool(overrides: Partial<ToolCall> = {}): ToolCall {
	return {
		id: "t1",
		title: "Edit file",
		status: "completed",
		kind: "other",
		content: "plain output",
		...overrides,
	}
}

function diffTool(
	overrides: Partial<ToolCall> & { path: string; oldText: string; newText: string },
): ToolCall {
	return tool({
		id: `t-${overrides.path}-${Math.random()}`,
		kind: "diff",
		title: "Edit file",
		content: {
			type: "diff",
			path: overrides.path,
			oldText: overrides.oldText,
			newText: overrides.newText,
		},
	})
}

function assistant(toolCalls: ToolCall[], content = ""): ChatMessage {
	return {
		id: crypto.randomUUID(),
		role: "assistant",
		content,
		toolCalls,
		timestamp: Date.now(),
	}
}

describe("isDiffTool", () => {
	it("detects diff kind", () => {
		expect(isDiffTool(tool({ kind: "diff" }))).toBe(true)
	})

	it("detects structured diff content", () => {
		expect(
			isDiffTool(
				tool({
					content: {
						type: "diff",
						path: "src/a.ts",
						oldText: "a",
						newText: "b",
					},
				}),
			),
		).toBe(true)
	})

	it("detects diff title fallback", () => {
		expect(isDiffTool(tool({ title: "Apply diff to App.tsx" }))).toBe(true)
	})
})

describe("collectDiffTools", () => {
	it("collects unique diff tools across messages", () => {
		const messages: ChatMessage[] = [
			{
				id: "m1",
				role: "assistant",
				content: "",
				timestamp: 1,
				toolCalls: [
					tool({ id: "d1", kind: "diff", title: "Diff 1" }),
					tool({ id: "d2", title: "Write diff" }),
				],
			},
			{
				id: "m2",
				role: "assistant",
				content: "",
				timestamp: 2,
				toolCalls: [tool({ id: "d1", kind: "diff", title: "Diff 1 dup" })],
			},
		]

		const diffs = collectDiffTools(messages)
		expect(diffs.map((d) => d.id)).toEqual(["d1", "d2"])
	})
})

describe("isGeneratedFile", () => {
	it("flags lockfiles and build output", () => {
		expect(isGeneratedFile("package-lock.json")).toBe(true)
		expect(isGeneratedFile("bun.lock")).toBe(true)
		expect(isGeneratedFile("dist/index.js")).toBe(true)
		expect(isGeneratedFile("src-tauri/target/debug/app")).toBe(true)
		expect(isGeneratedFile("src/App.tsx")).toBe(false)
		expect(isGeneratedFile("index.min.js")).toBe(true)
	})
})

describe("collectSessionDiffs", () => {
	it("groups diffs by path across the session", () => {
		const messages: ChatMessage[] = [
			assistant([
				diffTool({ path: "src/a.ts", oldText: "a1", newText: "a2" }),
				diffTool({ path: "src/b.ts", oldText: "b1", newText: "b2" }),
			]),
			assistant([diffTool({ path: "src/a.ts", oldText: "a2", newText: "a3" })]),
		]

		const diffs = collectSessionDiffs(messages)
		expect(diffs.map((d) => d.path)).toEqual(["src/a.ts", "src/b.ts"])
		const a = diffs.find((d) => d.path === "src/a.ts")!
		expect(a.newText).toBe("a3")
		expect(a.oldText).toBe("a1")
	})

	it("marks created files", () => {
		const messages: ChatMessage[] = [
			assistant([
				tool({
					id: "c1",
					kind: "diff",
					title: "Create new file",
					content: { type: "diff", path: "NEW.md", oldText: "", newText: "hi" },
				}),
			]),
		]
		const diffs = collectSessionDiffs(messages)
		expect(diffs[0]?.status).toBe("created")
		expect(diffs[0]?.generated).toBe(false)
	})

	it("flags generated files for auto-collapse", () => {
		const messages: ChatMessage[] = [
			assistant([
				diffTool({
					path: "package-lock.json",
					oldText: "v1",
					newText: "v2",
				}),
			]),
		]
		const diffs = collectSessionDiffs(messages)
		expect(diffs[0]?.generated).toBe(true)
	})
})
