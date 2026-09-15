import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * Top-level error boundary: catches React render errors and shows a recovery
 * screen instead of a blank window.
 */
export default class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[Lantern] Unhandled render error:", error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="flex flex-col items-center justify-center h-screen gap-4
                        bg-surface-0 text-neutral-400 p-8">
          <div className="text-4xl select-none">⚠</div>
          <div className="text-center max-w-md">
            <p className="text-sm font-semibold text-neutral-200 mb-1">
              Something went wrong
            </p>
            <p className="text-xs text-neutral-500 mb-4">
              {this.state.error.message}
            </p>
            <button
              onClick={() => this.setState({ error: null })}
              className="px-3 py-1.5 rounded bg-surface-3 hover:bg-surface-4
                         text-xs text-neutral-300 transition-colors"
            >
              Try again
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
