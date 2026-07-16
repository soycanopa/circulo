import type { FileDiffOptions } from "@pierre/diffs"

export const PIERRE_DIFF_OPTIONS: FileDiffOptions<undefined> = {
	theme: {
		dark: "github-dark-default",
		light: "github-dark-default",
	},
	themeType: "dark",
	diffStyle: "unified",
	overflow: "scroll",
	lineDiffType: "word",
	diffIndicators: "classic",
	disableLineNumbers: false,
}