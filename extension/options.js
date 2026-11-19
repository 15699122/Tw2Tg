const $ = (selector) => document.querySelector(selector);
const send = (message) => new Promise((resolve) => chrome.runtime.sendMessage(message, resolve));
const stateText = { disabled: "已关闭", unconfigured: "未配置", connecting: "连接中", connected: "已认证", disconnected: "已断开", auth_failed: "认证失败", auth_timeout: "认证超时", unavailable: "不可用", error: "错误" };
function setStatus(status) {
  const websocket = status?.websocket || {};
  $("#channel").textContent = status?.channel === "websocket" ? "WebSocket" : "Native Messaging";
  $("#websocket").textContent = `${stateText[websocket.state] || websocket.state || "未配置"}${websocket.port ? ` · ${websocket.port}` : ""}`;
}
async function refresh(observeOnly = false) { setStatus(await send({ type: "get_extension_status", observe_only: observeOnly })); }
$("#enabled").addEventListener("change", () => refresh());
$("#save").addEventListener("click", async () => {
  const button = $("#save"); button.disabled = true; $("#message").textContent = "保存中…";
  const result = await send({ type: "save_websocket_settings", settings: { enabled: $("#enabled").checked, mode: "automatic" } });
  const status = result?.status || result;
  button.disabled = false;
  if (status?.websocket?.enabled === false) {
    $("#message").textContent = "Desktop 自动连接已关闭。";
  } else if (result?.error) {
    $("#message").textContent = `保存失败：${result.error}`;
    return;
  } else {
    $("#message").textContent = "设置已保存，正在连接 Desktop。";
    setStatus(result.status);
    refresh();
  }
});
$("#reconnect").addEventListener("click", async () => { $("#message").textContent = "正在重新连接…"; setStatus(await send({ type: "reconnect_transport" })); $("#message").textContent = "已请求重新连接。"; });
(async () => { const status = await send({ type: "get_websocket_settings" }); $("#enabled").checked = status?.enabled === true; })();
chrome.runtime.onMessage.addListener((message, sender) => {
  if (sender.id === chrome.runtime.id && message?.type === "extension_status_changed") refresh(true).catch(() => {});
});
refresh();
