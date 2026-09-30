import { readWebSocketSettings, writeWebSocketSettings } from "./websocket-settings.js";

export const MAX_PENDING_REQUESTS = 64;
export const DEFAULT_AUTHENTICATION_TIMEOUT_MS = 5_000;
export const AUTHENTICATION_TIMEOUT_STATE = "auth_timeout";
export const DEFAULT_REQUEST_TIMEOUT_MS = 10_000;
export const DEFAULT_RETRY_DELAYS_MS = [500, 1000, 2000, 5000];

export class WebSocketBridge {
  constructor(api = globalThis.chrome, options = {}) {
    this.api = api;
    this.requestTimeoutMs = options.requestTimeoutMs ?? DEFAULT_REQUEST_TIMEOUT_MS;
    this.retryDelaysMs = options.retryDelaysMs ?? DEFAULT_RETRY_DELAYS_MS;
    this.authenticationTimeoutMs = options.authenticationTimeoutMs ?? DEFAULT_AUTHENTICATION_TIMEOUT_MS;
    this.socketFactory = options.socketFactory ?? globalThis.WebSocket;
    this.settings = { enabled: false, port: 17321, token: "" };
    this.socket = null;
    this.pending = new Map();
    this.retryTimer = null;
    this.retryIndex = 0;
    this.generation = 0;
    this.state = "unconfigured";
    this.lastError = null;
  }

  async loadSettings() {
    this.settings = await readWebSocketSettings(this.api);
    return { ...this.settings };
  }

  async saveSettings(value) {
    this.settings = await writeWebSocketSettings(this.api, value);
    this.disconnect("settings changed");
    return { ...this.settings };
  }

  getStatus() {
    return { transport: "websocket", state: this.state, enabled: this.settings.enabled, port: this.settings.port, authenticated: this.state === "connected", settingsConfigured: Boolean(this.settings.token), error: this.lastError };
  }

  connect() {
    if (this.socket?.readyState === 1) return Promise.resolve(this.socket);
    if (!this.settings.enabled) {
      this.state = "disabled";
      return Promise.reject(this.error("WebSocket is disabled", "WEBSOCKET_DISABLED", false));
    }
    if (!this.settings.token || typeof this.socketFactory !== "function") {
      this.state = this.settings.token ? "unavailable" : "unconfigured";
      return Promise.reject(this.error("WebSocket is not configured", "WEBSOCKET_NOT_CONFIGURED", false));
    }
    const generation = ++this.generation;
    this.state = "connecting";
    this.lastError = null;
    let socket;
    try { socket = new this.socketFactory(`ws://127.0.0.1:${this.settings.port}`); }
    catch (error) { this.state = "error"; return Promise.reject(this.error(error, "WEBSOCKET_CONNECT_FAILED")); }
    this.socket = socket;
    return new Promise((resolve, reject) => {
      let settled = false;
      let authenticationTimer;
      const clearAuthenticationTimeout = () => {
        if (authenticationTimer) { clearTimeout(authenticationTimer); authenticationTimer = null; }
      };
      const fail = (cause, code = "WEBSOCKET_CONNECTION_FAILED") => {
        if (settled || generation !== this.generation) return;
        settled = true;
        clearAuthenticationTimeout();
        this.state = code === "WEBSOCKET_AUTH_FAILED" ? "auth_failed" : code === "WEBSOCKET_AUTH_TIMEOUT" ? "auth_timeout" : "disconnected";
        this.lastError = String(cause?.message || cause);
        socket.onopen = null; socket.onmessage = null; socket.onerror = null; socket.onclose = null;
        try { socket.close(); } catch { /* connection is already unusable */ }
        this.socket = null;
        this.rejectPending(this.error(cause, code));
        if (!["WEBSOCKET_AUTH_FAILED", "WEBSOCKET_AUTH_TIMEOUT", "WEBSOCKET_NOT_CONFIGURED"].includes(code)) this.scheduleReconnect();
        reject(this.error(cause, code));
      };
      socket.onopen = () => {
        authenticationTimer = setTimeout(() => {
          fail(new Error("WebSocket authentication timed out"), "WEBSOCKET_AUTH_TIMEOUT");
        }, this.authenticationTimeoutMs);
        try { socket.send(JSON.stringify({ protocol_version: 1, message_type: "authenticate", token: this.settings.token })); }
        catch (cause) { fail(cause, "WEBSOCKET_AUTH_FAILED"); }
      };
      socket.onmessage = (event) => {
        let message;
        try { message = JSON.parse(event.data); } catch (cause) { fail(cause, "WEBSOCKET_PROTOCOL_ERROR"); return; }
        if (message?.message_type === "authentication_response") {
          if (message.authenticated !== true) { clearAuthenticationTimeout(); fail(new Error(message.error_code || "WebSocket authentication failed"), "WEBSOCKET_AUTH_FAILED"); return; }
          clearAuthenticationTimeout(); settled = true; this.state = "connected"; this.retryIndex = 0; resolve(socket); return;
        }
        if (generation === this.generation) this.handleMessage(message);
      };
      socket.onerror = () => fail(new Error("WebSocket connection failed"));
      socket.onclose = () => { if (!settled) fail(new Error("WebSocket connection closed")); else if (generation === this.generation) this.handleDisconnect(); };
    });
  }



  async send(message) {
    if (this.pending.size >= MAX_PENDING_REQUESTS) throw this.error("too many pending WebSocket requests", "WEBSOCKET_PENDING_LIMIT");
    if (this.pending.has(message.request_id)) throw this.error("duplicate WebSocket request_id", "DUPLICATE_REQUEST_ID", false);
    const socket = await this.connect();
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => this.rejectPending(message.request_id, this.error("WebSocket request timed out", "WEBSOCKET_REQUEST_TIMEOUT")), this.requestTimeoutMs);
      this.pending.set(message.request_id, { resolve, reject, timer });
      try { socket.send(JSON.stringify(message)); } catch (cause) { this.rejectPending(message.request_id, this.error(cause, "WEBSOCKET_REQUEST_FAILED")); }
    });
  }

  handleMessage(message) {
    if (message?.protocol_version !== 1 || typeof message.request_id !== "string") return;
    if (!["archive_status", "archive_status_batch", "error"].includes(message.message_type)) return;
    const waiter = this.pending.get(message.request_id);
    if (!waiter) return;
    this.pending.delete(message.request_id); clearTimeout(waiter.timer);
    if (message.message_type === "error") waiter.reject(this.error(message.error_message, message.error_code || "WEBSOCKET_ERROR", message.retryable !== false, message.request_id));
    else waiter.resolve(message);
  }

  handleDisconnect() {
    this.socket = null;
    if (this.state === "connected") this.state = "disconnected";
    this.rejectPending(this.error("WebSocket disconnected", "WEBSOCKET_DISCONNECTED"));
    this.scheduleReconnect();
  }

  disconnect(reason = "disconnected") {
    this.generation += 1;
    if (this.retryTimer) { clearTimeout(this.retryTimer); this.retryTimer = null; }
    this.retryIndex = 0;
    const socket = this.socket;
    this.socket = null;
    if (socket) {
      socket.onopen = null; socket.onmessage = null; socket.onerror = null; socket.onclose = null;
      try { socket.close(); } catch { /* connection is already unusable */ }
    }
    this.state = !this.settings.enabled ? "disabled" : reason === "settings changed" ? "unconfigured" : "disconnected";
    this.rejectPending(this.error(reason, "WEBSOCKET_DISCONNECTED"));
  }

  scheduleReconnect() {
    if (this.retryTimer || !this.settings.enabled || !this.settings.token) return;
    const delay = this.retryDelaysMs[Math.min(this.retryIndex++, this.retryDelaysMs.length - 1)];
    this.retryTimer = setTimeout(() => { this.retryTimer = null; this.connect().catch(() => {}); }, delay);
  }

  rejectPending(requestIdOrError, maybeError) {
    if (typeof requestIdOrError === "string") {
      const waiter = this.pending.get(requestIdOrError);
      if (!waiter) return false;
      this.pending.delete(requestIdOrError); clearTimeout(waiter.timer); waiter.reject(maybeError); return true;
    }
    for (const requestId of [...this.pending.keys()]) this.rejectPending(requestId, requestIdOrError);
    return true;
  }

  error(cause, code, retryable = true, requestId = null) {
    const result = new Error(String(cause?.message || cause || code));
    result.code = code; result.retryable = retryable; result.request_id = requestId;
    return result;
  }
}
