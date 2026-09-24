const $ = (selector) => document.querySelector(selector);
const send = (message) => new Promise((resolve) => chrome.runtime.sendMessage(message, resolve));
const labels = { disabled: "已关闭", unconfigured: "未配置", connecting: "连接中", connected: "已连接", disconnected: "已断开", auth_failed: "认证失败", unavailable: "不可用", error: "连接错误" };
function render(status, page) {
  const websocket = status?.websocket || {};
  const connected = status?.channel === "websocket" && websocket.state === "connected";
  const state = connected ? "已连接" : labels[websocket.state] || "未连接";
  $("#status-pill").textContent = state;
  $("#status-pill").className = `status-pill ${connected ? "ok" : websocket.state === "auth_failed" ? "error" : "warn"}`;
  $("#channel").textContent = status?.channel === "websocket" ? "WebSocket" : "Native Messaging 回退";
  $("#websocket-state").textContent = `${labels[websocket.state] || "未配置"}${websocket.port ? ` · 端口 ${websocket.port}` : ""}`;
  const supported = /^https:\/\/(x|twitter)\.com\//.test(page?.url || "");
  $("#page-state").textContent = supported ? "X / Twitter 页面" : "非归档页面";
  $("#page-action").textContent = supported ? "可用" : "不可用";
  $("#desktop-action").textContent = connected ? "可用" : status?.channel === "native" ? "Native 回退" : "等待连接";
  if (status?.websocket?.enabled === false) {
    $("#message").textContent = "WebSocket 已关闭；当前使用 Native Messaging（如已配置）。";
  } else if (websocket.error) {
    $("#message").textContent = websocket.error;
  } else if (connected) {
    $("#message").textContent = "Desktop 已认证，可以提交归档请求。";
  } else {
    $("#message").textContent = "请在设置中完成 Desktop 配对；迁移期间可使用 Native Messaging 回退。";
  }
  document.querySelectorAll(".dot").forEach((dot) => dot.className = `dot ${connected ? "ok" : "warn"}`);
}
async function refresh() { const [status, tab] = await Promise.all([send({ type: "get_extension_status" }), chrome.tabs.query({ active: true, currentWindow: true }).then(([tab]) => tab || {})]); render(status, tab); }
$("#open-options").addEventListener("click", () => chrome.runtime.openOptionsPage());
$("#reconnect").addEventListener("click", async () => { $("#reconnect").disabled = true; await send({ type: "reconnect_transport" }); await refresh(); $("#reconnect").disabled = false; });
refresh();
