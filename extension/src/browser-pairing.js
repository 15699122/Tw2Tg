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
