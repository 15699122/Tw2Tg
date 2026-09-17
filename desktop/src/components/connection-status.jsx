import { extensionSidebarState } from "../lib/ui-state.js";

export default function ConnectionStatus({ label, ready, loading, onClick }) {
  const tone = ready ? "dot-online" : loading ? "dot-muted" : "dot-error";
  const text = ready ? "已连接" : loading ? "检测中…" : "未连接";
  return (
    <button type="button" className="connection-line connection-line-button" onClick={onClick} aria-label={`${label}服务状态，打开设置`}>
      <i className={`dot ${tone}`} />
      <span>{label}</span>
      <small>{text}</small>
    </button>
  );
}

export function ExtensionConnectionStatus({ extension, initialLoad, onClick }) {
  const state = extensionSidebarState({
    filesReady: extension?.files_ready,
    browserConnection: extension?.browser_connection,
    initialLoad,
  });
  return (
    <button type="button" className="connection-line connection-line-button" onClick={onClick} aria-label="Extension服务状态，打开设置">
      <i className={`dot dot-${state.tone}`} />
      <span>Extension</span>
      <small>{state.text}</small>
    </button>
  );
}
