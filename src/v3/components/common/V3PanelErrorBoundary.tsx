import { Component, type ErrorInfo, type ReactNode } from "react";

interface V3PanelErrorBoundaryProps {
  children: ReactNode;
  resetKey: string;
  title?: string;
}

interface V3PanelErrorBoundaryState {
  error: Error | null;
}

export class V3PanelErrorBoundary extends Component<
  V3PanelErrorBoundaryProps,
  V3PanelErrorBoundaryState
> {
  state: V3PanelErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): V3PanelErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("V3 panel render failed", error, info.componentStack);
  }

  componentDidUpdate(previous: V3PanelErrorBoundaryProps) {
    if (this.state.error && previous.resetKey !== this.props.resetKey) {
      this.setState({ error: null });
    }
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm">
        <div className="font-semibold text-destructive">{this.props.title ?? "该面板暂时无法显示"}</div>
        <p className="mt-1 text-xs text-muted-foreground">
          单个面板渲染失败，项目界面仍可继续使用。切换任务或刷新后可重试。
        </p>
        <pre className="mt-3 max-h-28 overflow-auto whitespace-pre-wrap break-all rounded bg-background/70 p-2 text-[11px] text-destructive">
          {this.state.error.message}
        </pre>
      </div>
    );
  }
}
