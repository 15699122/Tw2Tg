const pairingErrorMessages = Object.freeze({
  DESKTOP_NOT_READY: "Desktop 尚未启动或尚未准备就绪。",
  NATIVE_PIPE_UNAVAILABLE: "无法连接 Desktop，请检查它是否启动及本机通信权限。",
  NATIVE_HOST_CONNECT_FAILED: "无法启动 Native Host，请检查浏览器注册。",
  NATIVE_HOST_DISCONNECTED: "Native Host 不可用或已断开，请检查浏览器注册。",
  BOOTSTRAP_PROTOCOL_ERROR: "配对信息不兼容，请检查 Desktop 和扩展版本。",
  WEBSOCKET_AUTH_FAILED: "Desktop 认证失败，请重新连接。",
  WEBSOCKET_AUTH_TIMEOUT: "Desktop 认证超时，请检查 Desktop 状态后重新连接。",
  WEBSOCKET_PROTOCOL_ERROR: "Desktop 连接协议不兼容，请检查应用版本。",
});

export function pairingErrorMessage(code) {
  if (!code) return "";
  return Object.hasOwn(pairingErrorMessages, code) ? pairingErrorMessages[code] : "连接失败，请检查 Desktop 和 Native Host 状态。";
}

// Bootstrap control is deliberately separate from business responses.
export function validBootstrap(message, requestId) {
  if (!message || typeof message !== "object") return false;
  const fields = ["protocol_version", "message_type", "request_id", "host", "port", "path", "runtime_instance_id", "ticket", "expires_in_ms"];
  return Object.keys(message).length === fields.length && fields.every((field) => Object.hasOwn(message, field))
    && message.protocol_version === 1 && message.message_type === "bootstrap" && message.request_id === requestId
    && message.host === "127.0.0.1" && Number.isInteger(message.port) && message.port > 0 && message.port <= 65535
    && message.path === "/" && typeof message.runtime_instance_id === "string"
    && /^[\x00-\x7f]{1,128}$/.test(message.runtime_instance_id)
    && typeof message.ticket === "string" && /^[0-9a-f]{64}$/.test(message.ticket)
    && Number.isInteger(message.expires_in_ms) && message.expires_in_ms > 0 && message.expires_in_ms <= 30000;
}

export function validPairingError(message, requestId) {
  const fields = ["protocol_version", "message_type", "request_id", "error_code", "error_message", "retryable"];
  return message && Object.keys(message).length === fields.length && fields.every((field) => Object.hasOwn(message, field))
    && message.protocol_version === 1 && message.message_type === "error" && message.request_id === requestId
    && typeof message.error_code === "string" && /^[\x21-\x7e]{1,64}$/.test(message.error_code)
    && typeof message.error_message === "string" && message.error_message.length > 0
    && new TextEncoder().encode(message.error_message).length <= 512 && !/[\x00-\x1f\x7f-\x9f]/.test(message.error_message)
    && typeof message.retryable === "boolean";
}
