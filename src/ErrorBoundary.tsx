import { Component, type ErrorInfo, type ReactNode } from "react";

type Props = { children: ReactNode };
type State = { err: Error | null };

export class ErrorBoundary extends Component<Props, State> {
  state: State = { err: null };

  static getDerivedStateFromError(err: Error): State {
    return { err };
  }

  componentDidCatch(err: Error, info: ErrorInfo) {
    console.error(err, info.componentStack);
  }

  render() {
    if (this.state.err) {
      return (
        <div style={{ padding: 24, fontFamily: "system-ui, sans-serif", color: "#14532d" }}>
          <h1 style={{ fontSize: 18 }}>界面渲染出错</h1>
          <pre
            style={{
              marginTop: 12,
              padding: 12,
              background: "#f0fdf4",
              border: "1px solid #bbf7d0",
              borderRadius: 8,
              whiteSpace: "pre-wrap",
              fontSize: 12,
            }}
          >
            {String(this.state.err)}
          </pre>
          <p style={{ marginTop: 16, fontSize: 13, color: "#166534" }}>
            数据默认在仓库内 <code>data/harness.db</code>（或见环境变量说明），与页面是否白屏无关。详见{" "}
            <code>docs/数据与运行说明.md</code>
          </p>
        </div>
      );
    }
    return this.props.children;
  }
}
