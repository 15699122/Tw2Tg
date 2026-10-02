import { validBootstrap, validPairingError } from "./browser-pairing.js";
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

  bootstrap() {
    return this.send({ protocol_version: 1, message_type: "bootstrap", request_id: createRequestId("bootstrap") });
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
      this.pending.set(message.request_id, { resolve, reject, timer, bootstrap: message.message_type === "bootstrap" });
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
    const waiter = this.pending.get(message?.request_id);
    if (!waiter) return;
    if (waiter.bootstrap && !(message?.message_type === "error"
      ? validPairingError(message, message.request_id) : validBootstrap(message, message.request_id))) {
      this.rejectPending(message.request_id, createBridgeError("invalid bootstrap response", "invalid bootstrap response", "BOOTSTRAP_PROTOCOL_ERROR", false));
      return;
    }
    if (!waiter.bootstrap && !isProtocolResponse(message)) return;
    if (message?.protocol_version !== 1) {
      this.rejectPending(message.request_id, createBridgeError("unsupported bootstrap version", "unsupported bootstrap version", "BOOTSTRAP_PROTOCOL_ERROR", false));
      return;
    }
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
    this.websocket = options.websocket || new WebSocketBridge(api, { ...options, bootstrap: () => this.native.bootstrap() });
    this.channel = options.compatibilityMode === "native" ? "native" : "websocket";
  }

  async initialize() {
    const settings = await this.websocket.loadSettings();
    if (settings.mode !== "native") {
      this.channel = "websocket";
      if (settings.enabled) await this.websocket.connect();
    } else this.channel = "native";
    return this.getStatus();
  }

  getStatus() {
    // A status read is the one moment the popup and the options page agree on
    // the truth, so it is also where a silently closed socket gets repaired.
    this.websocket.ensureConnected();
    return { channel: this.channel, native: { state: this.native.port ? "connected" : "not_connected" }, websocket: this.websocket.getStatus() };
  }

  async saveWebSocketSettings(value) {
    const settings = await this.websocket.saveSettings(value);
    this.channel = settings.mode === "native" ? "native" : "websocket";
    if (this.channel === "websocket" && settings.enabled) await this.websocket.connect();
    return { settings, status: this.getStatus() };
  }

  async reconnect() {
    this.websocket.disconnect("manual reconnect");
    if (this.websocket.settings.mode !== "native") {
      this.channel = "websocket";
      if (this.websocket.settings.enabled) await this.websocket.connect();
    } else this.channel = "native";
    return this.getStatus();
  }

  send(message) {
    // The guard follows the live socket, so a silently closed connection cannot
    // make the bridge believe a WebSocket send is possible.
    if (this.channel !== "websocket") return this.native.send(message);
    // Once WebSocket is selected, connection/authentication/protocol failures
    // must remain visible. Sending the same business request through Native
    // Messaging could bypass a rejected authentication or replay a request.
    return this.websocket.send(message);
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

export function authorizedSender(api, sender, message) {
  if (!sender || sender.id !== api.runtime.id || typeof sender.url !== "string") return false;
  let url;
  try { url = new URL(sender.url); } catch { return false; }
  const trustedPage = url.protocol === "chrome-extension:" && url.hostname === api.runtime.id
    && ["/popup.html", "/options.html"].includes(url.pathname);
  const content = url.protocol === "https:" && ["x.com", "twitter.com"].includes(url.hostname)
    && !url.port && !url.username && !url.password && Boolean(sender.tab);
  if (["get_websocket_settings", "save_websocket_settings", "reconnect_transport"].includes(message?.type)) return trustedPage;
  if (!["get_extension_status", "archive_request", "query_status"].includes(message?.type)) return false;
  if (!(trustedPage || content)) return false;
  if (message.type === "query_status") return Array.isArray(message.tweet_ids) && message.tweet_ids.length > 0 && message.tweet_ids.length <= 100
    && message.tweet_ids.every((id) => typeof id === "string" && /^[0-9]{1,32}$/.test(id));
  if (message.type === "archive_request") return message.tweet && typeof message.tweet === "object"
    && typeof message.tweet.tweet_id === "string" && /^[0-9]{1,32}$/.test(message.tweet.tweet_id)
    && typeof message.tweet.url === "string" && /^https:\/\/(x\.com|twitter\.com)\//.test(message.tweet.url);
  return true;
}

export function installBackground(api = globalThis.chrome, bridge = new TransportBridge(api)) {
  const ready = Promise.resolve(bridge.initialize?.()).catch(() => {});
  api.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (!authorizedSender(api, sender, message)) return false;
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
      ready.then(() => bridge.send(createArchiveRequest(message.tweet, message.request_id)))
        .then(sendResponse)
        .catch((error) => sendResponse(errorResponse(error, message.request_id)));
      return true;
    }
    if (message?.type === "query_status") {
      ready.then(() => bridge.send(createQueryStatusRequest(message.tweet_ids, message.request_id)))
        .then(sendResponse)
        .catch((error) => sendResponse(errorResponse(error, message.request_id)));
      return true;
    }
    return false;
  });
  return bridge;
}

if (typeof chrome !== "undefined" && chrome.runtime?.onMessage) installBackground();