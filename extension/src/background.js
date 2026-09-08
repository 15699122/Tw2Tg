export const NATIVE_HOST_NAME = "com.tw2tg.xarchive";
export const MAX_PENDING_REQUESTS = 64;

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
      typeof message.message_type === "string",
  );
}

export class NativeBridge {
  constructor(api = globalThis.chrome, hostName = NATIVE_HOST_NAME) {
    this.api = api;
    this.hostName = hostName;
    this.port = null;
    this.pending = new Map();
    this.reconnectTimer = null;
  }

  connect() {
    if (this.port) return this.port;
    this.port = this.api.runtime.connectNative(this.hostName);
    this.port.onMessage.addListener((message) => this.handleMessage(message));
    this.port.onDisconnect.addListener(() => this.handleDisconnect());
    return this.port;
  }

  send(message) {
    if (this.pending.size >= MAX_PENDING_REQUESTS) {
      return Promise.reject(new Error("too many pending Native Messaging requests"));
    }
    const port = this.connect();
    return new Promise((resolve, reject) => {
      this.pending.set(message.request_id, { resolve, reject });
      try {
        port.postMessage(message);
      } catch (error) {
        this.pending.delete(message.request_id);
        reject(error);
      }
    });
  }

  handleMessage(message) {
    if (!isProtocolResponse(message)) return;
    const waiter = this.pending.get(message.request_id);
    if (!waiter) return;
    this.pending.delete(message.request_id);
    if (message.message_type === "error") {
      waiter.reject(new Error(message.error_message || "Native Host error"));
    } else {
      waiter.resolve(message);
    }
  }

  handleDisconnect() {
    const error = this.port?.error?.message || "Native Host disconnected";
    for (const { reject } of this.pending.values()) reject(new Error(error));
    this.pending.clear();
    this.port = null;
    if (!this.reconnectTimer) {
      this.reconnectTimer = setTimeout(() => {
        this.reconnectTimer = null;
      }, 250);
    }
  }
}

export function installBackground(api = globalThis.chrome, bridge = new NativeBridge(api)) {
  api.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    if (message?.type === "archive_request") {
      bridge
        .send(createArchiveRequest(message.tweet, message.request_id))
        .then(sendResponse)
        .catch((error) => sendResponse({ message_type: "error", error_message: error.message }));
      return true;
    }
    if (message?.type === "query_status") {
      bridge
        .send(createQueryStatusRequest(message.tweet_ids, message.request_id))
        .then(sendResponse)
        .catch((error) => sendResponse({ message_type: "error", error_message: error.message }));
      return true;
    }
    return false;
  });
  return bridge;
}

if (typeof chrome !== "undefined" && chrome.runtime?.onMessage) installBackground();