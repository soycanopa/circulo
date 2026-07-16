import { DiffErrorBoundary } from "@/components/diff/diff-error-boundary"
import { PierrePatchDiff } from "@/components/diff/pierre-diff-view"
import { CodeBlock } from "@/components/chat/code-block"

interface MarkdownDiffBlockProps {
	code: string
}

export function MarkdownDiffBlock({ code }: MarkdownDiffBlockProps) {
	return (
		<DiffErrorBoundary fallback={<CodeBlock language="diff" code={code} variant="terminal" />}>
			<PierrePatchDiff patch={code} className="my-2" />
		</DiffErrorBoundary>
	)
}