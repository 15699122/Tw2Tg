import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./ui/button";
import SettingsSection from "./settings-section.jsx";
import { telegramSettingsInput } from "../lib/telegram-state.js";

export default function TelegramSettings({ expanded, onToggle }) {
  const [projection, setProjection] = useState(null);
  const [settings, setSettings] = useState(null);
  const [token, setToken] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  async function refresh() {
    const value = await invoke("get_telegram_settings");
    setProjection(value); setSettings(value.settings);
  }
  useEffect(() => { refresh().catch(() => setMessage("Telegram 状态不可用")); }, []);
  async function action(command, args = {}) {
    setBusy(true); setMessage("");
    try { const result = await invoke(command, args); setMessage(typeof result === "string" ? result : "操作完成"); }
    catch { setMessage("操作未完成，请检查凭据服务、设置及发送状态；未知结果不会自动重发。"); }
    finally { setToken(""); await refresh().catch(() => {}); setBusy(false); }
  }
  function field(key, value) { setSettings((current) => ({ ...current, [key]: value })); }
  return <SettingsSection
    id="telegram-settings"
    title="Telegram"
    description="归档与发送状态独立；Bot API 确认发送成功，不代表对方已接收或已读。"
    icon="telegram"
    expanded={expanded}
    onToggle={onToggle}
    className="telegram-settings-panel"
  >
    <div className="telegram-settings-content" aria-busy={busy}>
      {projection && <div className="telegram-service-status" role="status">
        <span>凭据服务<strong>{projection.provider_available ? "可用" : "未提供（无明文回退）"}</strong></span>
        <span>Bot Token<strong>{projection.settings.bot_token_present ? "已保存" : "未保存"}</strong></span>
        <span>Sender<strong>{projection.sender_running ? "运行中" : "已停止"}</strong></span>
      </div>}
      {settings && <fieldset className="telegram-settings-fields" disabled={busy}>
        <legend className="sr-only">Telegram 设置</legend>
        <div className="telegram-toggle-row">
          <label className="telegram-checkbox-field"><input type="checkbox" checked={settings.enabled} onChange={(e) => field("enabled", e.target.checked)} />启用 Telegram</label>
          <label className="telegram-checkbox-field"><input type="checkbox" checked={settings.auto_send_on_archive} onChange={(e) => field("auto_send_on_archive", e.target.checked)} />归档完成后自动入队</label>
        </div>
        <div className="settings-fields telegram-settings-grid">
          <div className="settings-field"><label htmlFor="telegram-endpoint-mode">Endpoint 模式</label><select id="telegram-endpoint-mode" value={settings.endpoint_mode} onChange={(e) => field("endpoint_mode", e.target.value)}><option value="cloud">Cloud HTTPS</option><option value="local">Local 回环服务</option></select></div>
          <div className="settings-field telegram-field-wide"><label htmlFor="telegram-api-base">API 地址</label><input id="telegram-api-base" type="url" value={settings.api_base} onChange={(e) => field("api_base", e.target.value)} /></div>
          <div className="settings-field"><label htmlFor="telegram-chat-id">Chat ID</label><input id="telegram-chat-id" value={settings.chat_id} onChange={(e) => field("chat_id", e.target.value)} /></div>
          <div className="settings-field"><label htmlFor="telegram-topic-id">Topic ID</label><input id="telegram-topic-id" type="number" value={settings.message_thread_id ?? ""} onChange={(e) => field("message_thread_id", e.target.value === "" ? null : Number(e.target.value))} /></div>
          <div className="settings-field"><label htmlFor="telegram-rotation-policy">同 bot 凭据轮换</label><select id="telegram-rotation-policy" value={settings.credential_rotation_resume_policy} onChange={(e) => field("credential_rotation_resume_policy", e.target.value)}><option value="automatic">自动续接已知待发项</option><option value="confirm">逐批确认</option></select></div>
          <div className="settings-field"><label htmlFor="telegram-upload-mode">媒体模式</label><select id="telegram-upload-mode" value={settings.upload_mode} onChange={(e) => field("upload_mode", e.target.value)}><option value="display">客户端展示（不承诺原文件一致）</option><option value="original_file">原文件文档</option></select></div>
          <div className="settings-field"><label htmlFor="telegram-connect-timeout">连接超时（秒）</label><input id="telegram-connect-timeout" type="number" min="1" max="3600" value={settings.connect_timeout_seconds} onChange={(e) => field("connect_timeout_seconds", Number(e.target.value))} /></div>
          <div className="settings-field"><label htmlFor="telegram-upload-timeout">上传处理超时（秒）</label><input id="telegram-upload-timeout" type="number" min="1" max="3600" value={settings.upload_processing_timeout_seconds} onChange={(e) => field("upload_processing_timeout_seconds", Number(e.target.value))} /></div>
          <div className="telegram-form-actions"><Button onClick={() => action("save_telegram_settings", { settings: telegramSettingsInput(settings) })}>保存设置</Button><Button variant="outline" onClick={() => { if (window.confirm("显式迁移端点：发送将持久化暂停。Cloud 源执行 logOut；Local 源先删除 webhook 再 close（启动前十分钟可能被拒绝）。失败或未知结果需人工核对，不会自动重试。继续？")) action("migrate_telegram_endpoint", { settings: telegramSettingsInput(settings), confirmed: true }); }}>确认并迁移端点…</Button></div>
        </div>
        {settings.migration_pending && <p role="status" className="settings-message settings-message-error">迁移未完成，发送保持暂停；核对服务状态后再明确确认，不要盲目重试。</p>}
      </fieldset>}
      <div className="telegram-credential-field settings-field"><label htmlFor="telegram-new-token">新 Bot Token</label><input id="telegram-new-token" type="password" autoComplete="new-password" value={token} disabled={busy || !projection?.provider_available} onChange={(e) => setToken(e.target.value)} /></div>
      <div className="telegram-actions-group">
        <div className="telegram-action-group">
          <span className="telegram-action-label">连接检查</span>
          <div className="button-row">
            <Button disabled={busy || !projection?.settings.bot_token_present} onClick={() => action("inspect_telegram_connection", { operation: "auth", confirmed: false })}>检查认证</Button>
            <Button disabled={busy || !projection?.settings.bot_token_present} onClick={() => action("inspect_telegram_connection", { operation: "target", confirmed: false })}>检查目标</Button>
            <Button disabled={busy || !projection?.settings.bot_token_present} onClick={() => { if (window.confirm("向已保存目标发送一条真实测试消息？未知结果不得自动重复。")) action("inspect_telegram_connection", { operation: "message", confirmed: true }); }}>发送测试消息…</Button>
            <Button disabled={busy || !token || !projection?.provider_available} onClick={() => action("replace_telegram_credential", { token })}>验证并替换凭据</Button>
            <Button variant="outline" disabled={busy || !projection?.provider_available} onClick={() => action("delete_telegram_credential")}>删除凭据</Button>
          </div>
        </div>
        <div className="telegram-action-group">
          <span className="telegram-action-label">发送服务</span>
          <div className="button-row">
            <Button variant="outline" disabled={busy || !projection?.provider_available} onClick={() => action("start_telegram_sender")}>启动发送</Button>
            <Button variant="outline" disabled={busy} onClick={() => action("stop_telegram_sender")}>停止发送</Button>
          </div>
        </div>
      </div>
      {!!projection?.resume_candidates.length && <Button disabled={busy} onClick={() => action("confirm_telegram_resume", { generation: projection.generation, outboxIds: projection.resume_candidates })}>确认本次轮换的 {projection.resume_candidates.length} 个候选项</Button>}
      {message && <p role="status" className={message.includes("未完成") || message.includes("不可用") ? "settings-message settings-message-error" : "settings-message"}>{message}</p>}
      {projection?.sender_error && <p role="status" className="settings-message settings-message-error">{projection.sender_error}</p>}
    </div>
  </SettingsSection>;
}