import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./ui/button";
import { CardTitle, CardDescription } from "./ui/card";
import { telegramSettingsInput } from "../lib/telegram-state.js";

export default function TelegramSettings() {
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
  return <section className="settings-section" id="telegram-settings" aria-busy={busy}>
    <div className="settings-section-header"><CardTitle>Telegram</CardTitle><CardDescription>归档与发送状态独立。仅 Bot API 确认表示发送成功，不表示已接收或已读。</CardDescription></div>
    <div className="settings-section-content">
      {projection && <p>凭据服务：{projection.provider_available ? "可用" : "未提供（无明文回退）"} · Token：{projection.settings.bot_token_present ? "已保存" : "未保存"} · Sender：{projection.sender_running ? "运行中" : "已停止"}</p>}
      {settings && <fieldset disabled={busy}>
        <label><input type="checkbox" checked={settings.enabled} onChange={(e) => field("enabled", e.target.checked)} />启用 Telegram</label>
        <label><input type="checkbox" checked={settings.auto_send_on_archive} onChange={(e) => field("auto_send_on_archive", e.target.checked)} />归档完成后自动入队</label>
        <label>Endpoint 模式<select value={settings.endpoint_mode} onChange={(e) => field("endpoint_mode", e.target.value)}><option value="cloud">Cloud HTTPS</option><option value="local">Local 回环服务</option></select></label>
        <label>API 地址<input value={settings.api_base} onChange={(e) => field("api_base", e.target.value)} /></label>
        <label>Chat ID<input value={settings.chat_id} onChange={(e) => field("chat_id", e.target.value)} /></label>
        <label>Topic ID<input type="number" value={settings.message_thread_id ?? ""} onChange={(e) => field("message_thread_id", e.target.value === "" ? null : Number(e.target.value))} /></label>
        <label>同 bot 凭据轮换<select value={settings.credential_rotation_resume_policy} onChange={(e) => field("credential_rotation_resume_policy", e.target.value)}><option value="automatic">自动续接已知待发项</option><option value="confirm">逐批确认</option></select></label>
        <label>媒体模式<select value={settings.upload_mode} onChange={(e) => field("upload_mode", e.target.value)}><option value="display">客户端展示（不承诺原文件一致）</option><option value="original_file">原文件文档</option></select></label>
        <label>连接超时（秒）<input type="number" min="1" max="3600" value={settings.connect_timeout_seconds} onChange={(e) => field("connect_timeout_seconds", Number(e.target.value))} /></label>
        <label>上传处理超时（秒）<input type="number" min="1" max="3600" value={settings.upload_processing_timeout_seconds} onChange={(e) => field("upload_processing_timeout_seconds", Number(e.target.value))} /></label>
        <Button onClick={() => action("save_telegram_settings", { settings: telegramSettingsInput(settings) })}>保存设置</Button>
        <Button onClick={() => { if (window.confirm("显式迁移端点：发送将持久化暂停。Cloud 源执行 logOut；Local 源先删除 webhook 再 close（启动前十分钟可能被拒绝）。失败或未知结果需人工核对，不会自动重试。继续？")) action("migrate_telegram_endpoint", { settings: telegramSettingsInput(settings), confirmed: true }); }}>确认并迁移端点…</Button>
        {settings.migration_pending && <p role="status">迁移未完成，发送保持暂停；核对服务状态后再明确确认，不要盲目重试。</p>}
      </fieldset>}
      <label>新 Bot Token<input type="password" autoComplete="off" value={token} disabled={busy || !projection?.provider_available} onChange={(e) => setToken(e.target.value)} /></label>
      <div className="button-row">
        <Button disabled={busy || !projection?.settings.bot_token_present} onClick={() => action("inspect_telegram_connection", { operation: "auth", confirmed: false })}>检查认证</Button>
        <Button disabled={busy || !projection?.settings.bot_token_present} onClick={() => action("inspect_telegram_connection", { operation: "target", confirmed: false })}>检查目标</Button>
        <Button disabled={busy || !projection?.settings.bot_token_present} onClick={() => { if (window.confirm("向已保存目标发送一条真实测试消息？未知结果不得自动重复。")) action("inspect_telegram_connection", { operation: "message", confirmed: true }); }}>发送测试消息…</Button>
        <Button disabled={busy || !token || !projection?.provider_available} onClick={() => action("replace_telegram_credential", { token })}>验证并替换凭据</Button>
        <Button variant="outline" disabled={busy || !projection?.provider_available} onClick={() => action("delete_telegram_credential")}>删除凭据</Button>
        <Button variant="outline" disabled={busy || !projection?.provider_available} onClick={() => action("start_telegram_sender")}>启动发送</Button>
        <Button variant="outline" disabled={busy} onClick={() => action("stop_telegram_sender")}>停止发送</Button>
      </div>
      {!!projection?.resume_candidates.length && <Button disabled={busy} onClick={() => action("confirm_telegram_resume", { generation: projection.generation, outboxIds: projection.resume_candidates })}>确认本次轮换的 {projection.resume_candidates.length} 个候选项</Button>}
      {message && <p role="status">{message}</p>}
      {projection?.sender_error && <p role="status">{projection.sender_error}</p>}
    </div>
  </section>;
}