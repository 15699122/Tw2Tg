import { WebSocketBridge } from "./websocket-bridge.js";

export const NATIVE_HOST_NAME = "com.tw2tg.xarchive";
export const MAX_PENDING_REQUESTS = 64;
export const DEFAULT_REQUEST_TIMEOUT_MS = 10_000;

export function createRequestId(prefix = "browser") {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

export function createArchiveRequest(tweet, requestId = createRequestId("archive")) {
  return {
    protocol_version: 1,
    request_id: requestId,
    message_type: "archive_request",
    tweet,
  };
}

export function createQueryStatusRequest(tweetIds, requestId = createRequestId("status")) {
  return {
    protocol_version: 1,
    request_id: requestId,
    message_type: "query_status",
    tweet_ids: [...new Set(tweetIds.map(String))],
  };
}

export function isProtocolResponse(message) {
  return Boolean(
    message &&
      message.protocol_version === 1 &&
      typeof message.request_id === "string" &&
      ["archive_status", "archive_status_batch", "error"].includes(message.message_type),
  );
}

export class NativeBridge {
  constructor(api = globalThis.chrome, hostName = NATIVE_HOST_NAME, options = {}) {
    this.api = api;
    this.hostName = hostName;
    this.requestTimeoutMs = options.requestTimeoutMs ?? DEFAULT_REQUEST_TIMEOUT_MS;
    this.port = null;
    this.pending = new Map();
    this.portGeneration = 0;
  }

  connect() {
    if (this.port) return this.port;
    let port;
    try {
      port = this.api.runtime.connectNative(this.hostName);
    } catch (error) {
      throw createBridgeError(error, "Native Host connection failed", "NATIVE_HOST_CONNECT_FAILED");
    }
    const generation = ++this.portGeneration;
    this.port = port;
    port.onMessage.addListener((message) => this.handleMessage(message, generation));
    port.onDisconnect.addListener(() => this.handleDisconnect(generation));
    return port;
  }

  send(message) {
    if (this.pending.size >= MAX_PENDING_REQUESTS) {
      return Promise.reject(createBridgeError(
        "too many pending Native Messaging requests",
        "too many pending Native Messaging requests",
        "NATIVE_PENDING_LIMIT",
      ));
    }
    if (this.pending.has(message.request_id)) {
      return Promise.reject(createBridgeError(
        `duplicate Native Messaging request_id: ${message.request_id}`,
        "duplicate Native Messaging request_id",
        "DUPLICATE_REQUEST_ID",
      ));
    }
    return new Promise((resolve, reject) => {
      let port;
      try {
        port = this.connect();
      } catch (error) {
        reject(error);
        return;
      }
      const timer = setTimeout(() => {
        const waiter = this.pending.get(message.request_id);
        if (!waiter) return;
        this.pending.delete(message.request_id);
        waiter.reject(createBridgeError(
          `Native Messaging request timed out: ${message.request_id}`,
          "Native Messaging request timed out",
          "NATIVE_REQUEST_TIMEOUT",
        ));
      }, this.requestTimeoutMs);
      this.pending.set(message.request_id, { resolve, reject, timer });
      try {
        port.postMessage(message);
      } catch (error) {
        this.rejectPending(message.request_id, createBridgeError(
          error,
          "Native Host request failed",
          "NATIVE_REQUEST_FAILED",
        ));
      }
    });
  }

  handleMessage(message, generation = this.portGeneration) {
    if (generation !== this.portGeneration) return;
    if (!isProtocolResponse(message)) return;
    const waiter = this.pending.get(message.request_id);
    if (!waiter) return;
    this.pending.delete(message.request_id);
    clearTimeout(waiter.timer);
    if (message.message_type === "error") {
      waiter.reject(createBridgeError(
        message.error_message || "Native Host error",
        message.error_message || "Native Host error",
        message.error_code || "NATIVE_HOST_ERROR",
        message.retryable !== false,
        message.request_id,
      ));
    } else {
      waiter.resolve(message);
    }
  }

  handleDisconnect(generation = this.portGeneration) {
    if (generation !== this.portGeneration) return;
    const runtimeError = this.api.runtime?.lastError;
    const error = createBridgeError(
      runtimeError || this.port?.error,
      "Native Host disconnected",
      "NATIVE_HOST_DISCONNECTED",
      true,
    );
    for (const requestId of this.pending.keys()) this.rejectPending(requestId, error);
    this.port = null;
  }

  rejectPending(requestId, error) {
    const waiter = this.pending.get(requestId);
    if (!waiter) return false;
    this.pending.delete(requestId);
    clearTimeout(waiter.timer);
    waiter.reject(error);
    return true;
  }
}

export class TransportBridge {
  constructor(api = globalThis.chrome, options = {}) {
    this.native = options.native || new NativeBridge(api);
    this.websocket = options.websocket || new WebSocketBridge(api, options);
    this.channel = "native";
  }

  async initialize() {
    const settings = await this.websocket.loadSettings();
    if (settings.enabled && settings.token) {
      this.channel = "websocket";
      await this.websocket.connect().catch(() => { this.channel = "native"; });
    }
    return this.getStatus();
  }

  getStatus() {
    return { channel: this.channel, native: { state: this.native.port ? "connected" : "not_connected" }, websocket: this.websocket.getStatus() };
  }

  async saveWebSocketSettings(value) {
    const settings = await this.websocket.saveSettings(value);
    this.channel = settings.enabled && settings.token ? "websocket" : "native";
    if (this.channel === "websocket") await this.websocket.connect().catch(() => { this.channel = "native"; });
    return { settings, status: this.getStatus() };
  }

  async reconnect() {
    this.websocket.disconnect("manual reconnect");
    if (this.websocket.settings.enabled && this.websocket.settings.token) {
      this.channel = "websocket";
      await this.websocket.connect().catch(() => { this.channel = "native"; });
    } else this.channel = "native";
    return this.getStatus();
  }

  send(message) {
    if (this.channel !== "websocket" || this.websocket.state !== "connected") return this.native.send(message);
    return this.websocket.send(message).catch((error) => {
      if (["WEBSOCKET_CONNECT_FAILED", "WEBSOCKET_AUTH_FAILED", "WEBSOCKET_NOT_CONFIGURED"].includes(error.code)) {
        this.channel = "native";
        return this.native.send(message);
      }
      throw error;
    });
  }
}

export function normalizeNativeError(error, fallback = "Native Host error") {
  const message = typeof error === "string" ? error : error?.message;
  return String(message || fallback);
}

export function createBridgeError(error, fallback, code, retryable = true, requestId = null) {
  const bridgeError = new Error(normalizeNativeError(error, fallback));
  bridgeError.code = code;
  bridgeError.retryable = retryable;
  bridgeError.request_id = requestId;
  return bridgeError;
}

function errorResponse(error, requestId) {
  return {
    protocol_version: 1,
    request_id: error.request_id || requestId || null,
    message_type: "error",
    error_code: error.code || "NATIVE_HOST_ERROR",
    error_message: error.message || "Native Host error",
    retryable: error.retryable !== false,
  };
}

export function installBackground(api = globalThis.chrome, bridge = new TransportBridge(api)) {
  const ready = bridge.initialize?.() || Promise.resolve();
  api.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    if (message?.type === "get_extension_status") {
      ready.then(() => sendResponse(bridge.getStatus())).catch((error) => sendResponse({ channel: "unknown", error: String(error) }));
      return true;
    }
    if (message?.type === "get_websocket_settings") {
      ready.then(() => sendResponse(bridge.websocket.getStatus())).catch((error) => sendResponse({ error: String(error) }));
      return true;
    }
    if (message?.type === "save_websocket_settings") {
      ready.then(() => bridge.saveWebSocketSettings(message.settings)).then(sendResponse).catch((error) => sendResponse({ error: String(error) }));
      return true;
    }
    if (message?.type === "reconnect_transport") {
      ready.then(() => bridge.reconnect()).then(sendResponse).catch((error) => sendResponse({ error: String(error) }));
      return true;
    }
    if (message?.type === "archive_request") {
      bridge
        .send(createArchiveRequest(message.tweet, message.request_id))
        .then(sendResponse)
        .catch((error) => sendResponse(errorResponse(error, message.request_id)));
      return true;
    }
    if (message?.type === "query_status") {
      bridge
        .send(createQueryStatusRequest(message.tweet_ids, message.request_id))
        .then(sendResponse)
        .catch((error) => sendResponse(errorResponse(error, message.request_id)));
      return true;
    }
    return false;
  });
  return bridge;
}

if (typeof chrome !== "undefined" && chrome.runtime?.onMessage) installBackground();