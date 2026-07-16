import { DiffErrorBoundary } from "@/components/diff/diff-error-boundary"
import { NaiveDiffBlock } from "@/components/diff/naive-diff-block"
import { PierreFileDiff } from "@/components/diff/pierre-diff-view"

interface InlineDiffBlockProps {
	path: string
	oldText?: string
	newText: string
}

export function InlineDiffBlock({ path, oldText, newText }: InlineDiffBlockProps) {
	return (
		<div className="my-2">
			<DiffErrorBoundary
				fallback={<NaiveDiffBlock path={path} oldText={oldText} newText={newText} />}
			>
				<PierreFileDiff path={path} oldText={oldText} newText={newText} />
			</DiffErrorBoundary>
		</div>
	)
}