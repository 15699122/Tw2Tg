import React from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./ui/button";

/**
 * Page-level error boundary. A runtime error inside one page must never blank
 * the whole window (the settings white-screen regression); it renders an
 * actionable error panel with retry / back-to-dashboard / copy-details actions
 * instead.
 */
export default class ErrorBoundary extends React.Component {
  constructor(props) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error) {
    return { error };
  }

  componentDidCatch(error, info) {
    console.error("[xarchive] page render error:", error, info?.componentStack);
  }

  render() {
    const { error } = this.state;
    if (!error) {
      return this.props.children;
    }
    const message = String(error?.message || error || "unknown error");
    const copyDetails = () => {
      invoke("copy_text_to_clipboard", { text: message }).catch(() => {});
    };
    return (
      <div className="page-error" role="alert">
        <strong>页面出现错误</strong>
        <p>当前页面渲染失败，已阻止整页白屏。错误信息：{message}</p>
        <div className="button-row">
          <Button size="sm" onClick={() => this.setState({ error: null })}>重试</Button>
          <Button size="sm" variant="outline" onClick={() => this.props.setPage?.("dashboard")}>返回工作台</Button>
          <Button size="sm" variant="outline" onClick={copyDetails}>复制错误信息</Button>
        </div>
      </div>
    );
  }
}