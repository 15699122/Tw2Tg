import { extensionSidebarState } from "../lib/ui-state.js";

export default function ConnectionStatus({ label, ready, loading }) {
  const tone = ready ? "dot-online" : loading ? "dot-muted" : "dot-error";
  const text = ready ? "已连接" : loading ? "检测中…" : "未连接";
  return (
    <div className="connection-line">
      <i className={`dot ${tone}`} />
      <span>{label}</span>
      <small>{text}</small>
    </div>
  );
}

export function ExtensionConnectionStatus({ extension, initialLoad }) {
  const state = extensionSidebarState({
    filesReady: extension?.files_ready,
    browserConnection: extension?.browser_connection,
    initialLoad,
  });
  return (
    <div className="connection-line">
      <i className={`dot dot-${state.tone}`} />
      <span>Extension</span>
      <small>{state.text}</small>
    </div>
  );
}
