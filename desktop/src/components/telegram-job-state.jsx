import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { telegramActions } from "../lib/telegram-state.js";

export default function TelegramJobState({ tweetId }) {
  const [rows, setRows] = useState([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let disposed = false;
    const refresh = () => invoke("get_telegram_job_state", { tweetId }).then((value) => {
      if (!disposed) { setRows(value); setError(""); }
    }).catch(() => { if (!disposed) setError("Telegram 状态不可用"); });
    refresh(); const timer = setInterval(refresh, 5000);
    return () => { disposed = true; clearInterval(timer); };
  }, [tweetId]);
  async function cancel(row) {
    setBusy(true);
    try {
      await invoke("cancel_telegram_send", { botIdentity: row.bot_identity, idempotencyKey: row.idempotency_key });
      setRows(await invoke("get_telegram_job_state", { tweetId }));
    } catch { setError("取消未完成，请刷新确认状态"); }
    finally { setBusy(false); }
  }
  async function review(row) {
    if (!window.confirm("已人工核对目标消息，仍决定重发？可能产生重复消息。")) return;
    setBusy(true);
    try {
      const current = await invoke("get_telegram_settings");
      if (!current.generation || current.bot_identity !== row.bot_identity) throw new Error("changed");
      await invoke("review_telegram_unknown", { generation: current.generation, outboxId: row.id, confirmed: true });
      setRows(await invoke("get_telegram_job_state", { tweetId }));
    } catch { setError("复核未完成，请刷新确认目标及凭据"); }
    finally { setBusy(false); }
  }
  return <div className="telegram-job-state" aria-label="Telegram 发送状态">
    {error && <small role="status">{error}</small>}
    {rows.map((row) => <div key={row.id}><small>{row.label}</small>
      {row.progress && <small role="status"> · {row.progress}</small>}
      {row.reason && <small> · {row.reason}</small>}
      {telegramActions(row.state).cancel && <button disabled={busy} onClick={() => cancel(row)}>取消发送</button>}
      {telegramActions(row.state).review && <small>请先核对目标消息；不会自动重发。</small>}
      {telegramActions(row.state).review && <button disabled={busy} onClick={() => review(row)}>人工复核后重发…</button>}
      {row.message_link && <a href={row.message_link} target="_blank" rel="noreferrer">打开已确认消息</a>}
    </div>)}
  </div>;
}