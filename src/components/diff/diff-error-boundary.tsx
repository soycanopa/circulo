import { Component, type ErrorInfo, type ReactNode } from "react"

interface DiffErrorBoundaryProps {
	fallback: ReactNode
	children: ReactNode
}

interface DiffErrorBoundaryState {
	hasError: boolean
}

export class DiffErrorBoundary extends Component<
	DiffErrorBoundaryProps,
	DiffErrorBoundaryState
> {
	state: DiffErrorBoundaryState = { hasError: false }

	static getDerivedStateFromError(): DiffErrorBoundaryState {
		return { hasError: true }
	}

	componentDidCatch(error: Error, info: ErrorInfo) {
		console.warn("[diff] Pierre render failed, using fallback.", error, info.componentStack)
	}

	componentDidUpdate(prevProps: DiffErrorBoundaryProps) {
		if (prevProps.children !== this.props.children && this.state.hasError) {
			this.setState({ hasError: false })
		}
	}

	render() {
		if (this.state.hasError) return this.props.fallback
		return this.props.children
	}
}