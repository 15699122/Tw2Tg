const MAX_EVENT_NAME_LENGTH = 64;
const MAX_MESSAGE_LENGTH = 512;
const MAX_CONTEXT_LENGTH = 2048;

let startupState = "document_loaded";
let fallbackTimer = null;

function truncate(value, limit) {
  return String(value ?? "").slice(0, limit);
}

function safeError(error) {
  if (error instanceof Error) {
    return { message: truncate(error.message, MAX_MESSAGE_LENGTH), stack: truncate(error.stack, MAX_CONTEXT_LENGTH) };
  }
  return { message: truncate(error, MAX_MESSAGE_LENGTH), stack: "" };
}

export function setStartupState(state) {
  startupState = truncate(state, MAX_EVENT_NAME_LENGTH) || "unknown";
  document.documentElement.dataset.xarchiveStartup = startupState;
  window.dispatchEvent(new CustomEvent("xarchive:startup", { detail: startupState }));
}

export function getStartupState() {
  return startupState;
}

export function emitFrontendEvent(event, details = {}) {
  const payload = {
    event: truncate(event, MAX_EVENT_NAME_LENGTH),
    state: startupState,
    message: truncate(details.message, MAX_MESSAGE_LENGTH),
    context: truncate(details.context, MAX_CONTEXT_LENGTH),
    source: truncate(details.source, MAX_CONTEXT_LENGTH),
  };

  window.dispatchEvent(new CustomEvent("xarchive:diagnostic", { detail: payload }));
  // Do not make startup depend on the Tauri bridge: diagnostic logging is best effort.
  import("@tauri-apps/api/core")
    .then(({ invoke }) => invoke("log_frontend_event", { event: payload }))
    .catch(() => {});
}

export function installFrontendBootstrap() {
  setStartupState("document_loaded");
  window.addEventListener("error", (event) => {
    const error = safeError(event.error || event.message);
    emitFrontendEvent("window_error", {
      ...error,
      context: `readyState=${document.readyState};source=${event.filename || ""}:${event.lineno || 0}:${event.colno || 0}`,
    });
    showStartupFailure("前端启动失败", error.message);
  });
  window.addEventListener("unhandledrejection", (event) => {
    const error = safeError(event.reason);
    emitFrontendEvent("unhandled_rejection", error);
    showStartupFailure("前端初始化失败", error.message);
  });
  fallbackTimer = window.setTimeout(() => {
    if (document.documentElement.dataset.xarchiveStartup !== "react_mount_completed") {
      emitFrontendEvent("startup_timeout", { context: `state=${startupState};readyState=${document.readyState}` });
      showStartupFailure("XArchive 启动超时", "界面未能完成加载，请查看 logs 目录中的应用日志。");
    }
  }, 15000);
}

export function markReactMounted() {
  setStartupState("react_mount_completed");
  if (fallbackTimer !== null) {
    window.clearTimeout(fallbackTimer);
    fallbackTimer = null;
  }
  document.getElementById("startup-fallback")?.remove();
}

export function showStartupFailure(title, message) {
  const fallback = document.getElementById("startup-fallback");
  if (!fallback) return;
  fallback.dataset.state = "error";
  const titleNode = fallback.querySelector("strong");
  const messageNode = fallback.querySelector("span");
  if (titleNode) titleNode.textContent = title;
  if (messageNode) messageNode.textContent = truncate(message, MAX_MESSAGE_LENGTH);
}
