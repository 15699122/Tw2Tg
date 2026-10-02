import { validBootstrap } from "./browser-pairing.js";
import { readWebSocketSettings, writeWebSocketSettings } from "./websocket-settings.js";

export const MAX_PENDING_REQUESTS = 64;
export const DEFAULT_AUTHENTICATION_TIMEOUT_MS = 5_000;
export const AUTHENTICATION_TIMEOUT_STATE = "auth_timeout";
export const DEFAULT_REQUEST_TIMEOUT_MS = 10_000;
export const DEFAULT_RETRY_DELAYS_MS = [500, 1000, 2000, 5000];

/** `WebSocket.OPEN`; a numeric literal keeps this module usable without a DOM. */
export const SOCKET_OPEN = 1;

export class WebSocketBridge {
  constructor(api = globalThis.chrome, options = {}) {
    this.api = api;
    this.bootstrap = options.bootstrap ?? null;
    this.connectPromise = null;
    this.cancelConnect = null;
    this.requestTimeoutMs = options.requestTimeoutMs ?? DEFAULT_REQUEST_TIMEOUT_MS;
    this.retryDelaysMs = options.retryDelaysMs ?? DEFAULT_RETRY_DELAYS_MS;
    this.authenticationTimeoutMs = options.authenticationTimeoutMs ?? DEFAULT_AUTHENTICATION_TIMEOUT_MS;
    this.socketFactory = options.socketFactory ?? globalThis.WebSocket;
    this.settings = { enabled: false, port: 17321, token: "" };
    this.socket = null;
    this.endpointPort = null;
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

  /**
   * The state a reader may act on.
   *
   * `this.state` alone is a cached flag: a socket can be gone without
   * `onclose` having run — for example when the browser tears down the
   * background worker that owned it. Reporting that stale value would tell
   * the user "authenticated" while nothing is connected, which is exactly the
   * disagreement the two sides must never show. Deriving the reported state
   * from the live socket keeps the answer true regardless of the cause.
   */
  liveState() {
    if (this.state === "connected" && this.socket?.readyState !== SOCKET_OPEN) return "disconnected";
    return this.state;
  }

  /**
   * Re-establish the connection when the bridge is configured but has no live
   * socket. A silent close leaves no timer behind, so the next status read
   * heals the state instead of waiting for a manual reconnect.
   */
  ensureConnected() {
    if (!this.settings.enabled || this.settings.mode === "native" || (!this.bootstrap && !this.settings.token)) return false;
    if (this.liveState() === "connected" || this.connectPromise) return false;
    if (this.retryTimer || ["auth_failed", "auth_timeout", "protocol_error"].includes(this.state)) return false;
    return this.scheduleReconnect();
  }

  getStatus() {
    const state = this.liveState();
    return { transport: "websocket", state, enabled: this.settings.enabled, port: this.endpointPort, authenticated: state === "connected", settingsConfigured: Boolean(this.bootstrap || this.settings.token), error: this.lastError };
  }

  connect() {
    if (this.liveState() === "connected") return Promise.resolve(this.socket);
    if (this.connectPromise) return this.connectPromise;
    const generation = ++this.generation;
    let cancel;
    const cancelled = new Promise((_, reject) => { cancel = () => reject(this.error("connection cancelled", "WEBSOCKET_DISCONNECTED")); });
    this.cancelConnect = cancel;
    const attempt = Promise.resolve().then(async () => {
      if (!this.settings.enabled) { this.state = "disabled"; throw this.error("WebSocket is disabled", "WEBSOCKET_DISABLED", false); }
      let credentials = null;
      if (this.bootstrap && this.settings.mode !== "legacy") {
        this.state = "discovering";
        credentials = await this.bootstrap();
        if (!validBootstrap(credentials, credentials?.request_id)) throw this.error("invalid bootstrap response", "BOOTSTRAP_PROTOCOL_ERROR", false);
      }
      if (generation !== this.generation) throw this.error("connection cancelled", "WEBSOCKET_DISCONNECTED");
      const socket = await this.openSocket(generation, credentials);
      if (credentials && generation === this.generation) {
        try {
          const settings = await writeWebSocketSettings(this.api, { enabled: this.settings.enabled, mode: "automatic" });
          if (generation !== this.generation) throw this.error("connection cancelled", "WEBSOCKET_DISCONNECTED");
          this.settings = settings;
        } catch {
          if (generation === this.generation) {
            this.disconnect("credential migration failed");
            this.state = "protocol_error";
            this.lastError = "SETTINGS_MIGRATION_FAILED";
          }
          throw this.error("credential migration failed", "SETTINGS_MIGRATION_FAILED", false);
        }
      }
      return socket;
    });
    const promise = Promise.race([attempt, cancelled]).catch((error) => {
      if (generation === this.generation && this.state === "discovering") {
        this.state = "disconnected";
        this.lastError = error.code || "BOOTSTRAP_FAILED";
        if (error.retryable !== false) this.scheduleReconnect();
      }
      throw error;
    }).finally(() => {
      if (this.connectPromise === promise) { this.connectPromise = null; this.cancelConnect = null; }
    });
    this.connectPromise = promise;
    return promise;
  }

  openSocket(generation, credentials) {
    if (!this.settings.enabled) {
      this.state = "disabled";
      return Promise.reject(this.error("WebSocket is disabled", "WEBSOCKET_DISABLED", false));
    }
    if ((!credentials && !this.settings.token) || typeof this.socketFactory !== "function") {
      this.state = this.settings.token ? "unavailable" : "unconfigured";
      return Promise.reject(this.error("WebSocket is not configured", "WEBSOCKET_NOT_CONFIGURED", false));
    }
    this.state = "connecting";
    this.lastError = null;
    let socket;
    try { socket = new this.socketFactory(credentials ? `ws://127.0.0.1:${credentials.port}/` : `ws://127.0.0.1:${this.settings.port}`); }
    catch (error) { this.state = "error"; return Promise.reject(this.error(error, "WEBSOCKET_CONNECT_FAILED")); }
    this.socket = socket;
    this.endpointPort = credentials?.port ?? this.settings.port;
    return new Promise((resolve, reject) => {
      let settled = false;
      let authenticationTimer;
      const clearAuthenticationTimeout = () => {
        if (authenticationTimer) { clearTimeout(authenticationTimer); authenticationTimer = null; }
      };
      this.clearAuthenticationTimeout = clearAuthenticationTimeout;
      const fail = (cause, code = "WEBSOCKET_CONNECTION_FAILED") => {
        if (generation !== this.generation) return;
        const authenticated = settled;
        settled = true;
        clearAuthenticationTimeout();
        this.state = code === "WEBSOCKET_AUTH_FAILED" ? "auth_failed" : code === "WEBSOCKET_AUTH_TIMEOUT" ? "auth_timeout" : code === "WEBSOCKET_PROTOCOL_ERROR" ? "protocol_error" : "disconnected";
        this.lastError = code;
        socket.onopen = null; socket.onmessage = null; socket.onerror = null; socket.onclose = null;
        try { socket.close(); } catch { /* connection is already unusable */ }
        this.socket = null;
        this.rejectPending(this.error(code, code));
        if (!["WEBSOCKET_AUTH_FAILED", "WEBSOCKET_AUTH_TIMEOUT", "WEBSOCKET_NOT_CONFIGURED", "WEBSOCKET_PROTOCOL_ERROR"].includes(code)) this.scheduleReconnect();
        if (!authenticated) reject(this.error(code, code));
      };
      authenticationTimer = setTimeout(() => {
          fail(new Error("WebSocket authentication timed out"), "WEBSOCKET_AUTH_TIMEOUT");
        }, this.authenticationTimeoutMs);
      socket.onopen = () => {
        if (generation !== this.generation) return;
        const authentication = credentials ? { protocol_version: 1, message_type: "authenticate", ticket: credentials.ticket } : { protocol_version: 1, message_type: "authenticate", token: this.settings.token };
        try { socket.send(JSON.stringify(authentication)); if (credentials) credentials.ticket = ""; }
        catch (cause) { fail(cause, "WEBSOCKET_AUTH_FAILED"); }
      };
      socket.onmessage = (event) => {
        if (generation !== this.generation) return;
        let message;
        try { message = JSON.parse(event.data); } catch (cause) { fail(cause, "WEBSOCKET_PROTOCOL_ERROR"); return; }
        if (message?.message_type === "authentication_response") {
          if (settled || message.protocol_version !== 1 || Object.keys(message).some((key) => !["protocol_version", "message_type", "authenticated", "error_code"].includes(key)) || message.error_code && message.authenticated === true) {
            fail(new Error("invalid authentication response"), "WEBSOCKET_PROTOCOL_ERROR"); return;
          }
          if (message.authenticated !== true) { clearAuthenticationTimeout(); fail(new Error("WebSocket authentication failed"), "WEBSOCKET_AUTH_FAILED"); return; }
          clearAuthenticationTimeout(); settled = true; this.state = "connected"; this.retryIndex = 0; resolve(socket); return;
        }
        if (!settled) { fail(new Error("business response before authentication"), "WEBSOCKET_PROTOCOL_ERROR"); return; }
        this.handleMessage(message);
      };
      socket.onerror = () => fail(new Error("WebSocket connection failed"));
      socket.onclose = () => { if (!settled) fail(new Error("WebSocket connection closed")); else if (generation === this.generation) this.handleDisconnect(); };
    });
  }



  async send(message) {
    if (this.pending.size >= MAX_PENDING_REQUESTS) throw this.error("too many pending WebSocket requests", "WEBSOCKET_PENDING_LIMIT");
    if (this.pending.has(message.request_id)) throw this.error("duplicate WebSocket request_id", "DUPLICATE_REQUEST_ID", false);
    const socket = await this.connect();
    if (this.pending.size >= MAX_PENDING_REQUESTS) throw this.error("too many pending WebSocket requests", "WEBSOCKET_PENDING_LIMIT");
    if (this.pending.has(message.request_id)) throw this.error("duplicate WebSocket request_id", "DUPLICATE_REQUEST_ID", false);
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
    this.cancelConnect?.();
    this.clearAuthenticationTimeout?.();
    this.connectPromise = null;
    this.cancelConnect = null;
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
    if (this.retryTimer || !this.settings.enabled || this.settings.mode === "native" || (!this.bootstrap && !this.settings.token) || this.retryIndex >= this.retryDelaysMs.length) return false;
    const delay = this.retryDelaysMs[Math.min(this.retryIndex++, this.retryDelaysMs.length - 1)];
    this.retryTimer = setTimeout(() => { this.retryTimer = null; this.connect().catch(() => {}); }, delay);
    return true;
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
