import test from "node:test";
import assert from "node:assert/strict";
import { NativeBridge, TransportBridge, authorizedSender, installBackground } from "../src/background.js";
import { WebSocketBridge } from "../src/websocket-bridge.js";
import { validBootstrap, validPairingError } from "../src/browser-pairing.js";
import { normalizeWebSocketSettings } from "../src/websocket-settings.js";

function credentials(requestId = "r1", port = 43127) {
  return { protocol_version: 1, message_type: "bootstrap", request_id: requestId,
    host: "127.0.0.1", port, path: "/", runtime_instance_id: "runtime", ticket: "a".repeat(64), expires_in_ms: 30000 };
}
function event() {
  const listeners = [];
  return { addListener(listener) { listeners.push(listener); }, emit(value) { for (const listener of listeners) listener(value); } };
}
function storageApi(initial) {
  let stored = initial;
  return { runtime: {}, storage: { local: {
    get(key, callback) { callback({ [key]: stored }); },
    set(values, callback) { stored = Object.values(values)[0]; callback(); },
  } }, stored: () => stored };
}
function socketClass(sockets, mode = "accept") {
  return class {
    constructor(url) { this.url = url; this.readyState = 0; this.sent = []; sockets.push(this);
      queueMicrotask(() => { this.readyState = 1; this.onopen?.(); }); }
    send(payload) {
      const message = JSON.parse(payload);
      this.sent.push(message);
      if (message.message_type === "authenticate" && mode !== "silent") {
        queueMicrotask(() => this.onmessage?.({ data: JSON.stringify(mode === "business" ?
          { protocol_version: 1, message_type: "archive_status", request_id: "r1" } :
          { protocol_version: mode === "version" ? 2 : 1, message_type: "authentication_response", authenticated: mode !== "reject" }) }));
      }
    }
    close() { this.readyState = 3; this.onclose?.(); }
  };
}
test("bootstrap validates strict loopback fields, versions, ticket and TTL", () => {
  assert.equal(validBootstrap(credentials(), "r1"), true);
  for (const patch of [{ protocol_version: 2 }, { host: "localhost" }, { port: 0 }, { port: 65536 },
    { path: "/?ticket=x" }, { ticket: "A".repeat(64) }, { expires_in_ms: 30001 },
    { request_id: "other" }, { unexpected: true }, { runtime_instance_id: "非ASCII" }]) {
    assert.equal(validBootstrap({ ...credentials(), ...patch }, "r1"), false);
  }
});
test("Native bootstrap keeps control separate and rejects malformed matching replies", async () => {
  for (const malformed of [false, true]) {
    const port = { onMessage: event(), onDisconnect: event(), postMessage(message) {
      assert.equal(message.message_type, "bootstrap");
      this.onMessage.emit({ ...credentials(message.request_id), ...(malformed ? { host: "remote" } : {}) });
    } };
    const bridge = new NativeBridge({ runtime: { connectNative: () => port } });
    if (malformed) await assert.rejects(bridge.bootstrap(), (error) => error.code === "BOOTSTRAP_PROTOCOL_ERROR");
    else assert.equal((await bridge.bootstrap()).host, "127.0.0.1");
    assert.equal(bridge.pending.size, 0);
  }
});
test("automatic connect is single flight before bootstrap and authentication", async () => {
  let release, calls = 0;
  const sockets = [];
  const api = storageApi({ enabled: true, token: "legacy-secret", port: 17321 });
  const bridge = new WebSocketBridge(api, { socketFactory: socketClass(sockets), bootstrap: () => {
    calls += 1; return new Promise((resolve) => { release = resolve; });
  } });
  await bridge.loadSettings();
  const first = bridge.connect(), second = bridge.connect();
  assert.equal(first, second);
  await Promise.resolve();
  assert.equal(calls, 1);
  assert.equal(bridge.getStatus().authenticated, false);
  release(credentials());
  await first;
  assert.equal(sockets.length, 1);
  assert.equal(sockets[0].url, "ws://127.0.0.1:43127/");
  assert.equal(sockets[0].url.includes("ticket"), false);
  assert.equal(sockets[0].sent[0].ticket.length, 64);
  assert.equal(sockets[0].sent[0].token, undefined);
  assert.equal(JSON.stringify(api.stored()).includes("legacy-secret"), false);
  assert.equal(JSON.stringify(api.stored()).includes("a".repeat(64)), false);
  bridge.disconnect();
});
test("cancelled discovery and stale callbacks cannot replace a newer socket", async () => {
  let release, calls = 0;
  const sockets = [];
  const bridge = new WebSocketBridge(storageApi(), { socketFactory: socketClass(sockets), bootstrap: () => {
    calls += 1;
    return calls === 1 ? new Promise((resolve) => { release = resolve; }) : Promise.resolve(credentials("r2", 43128));
  } });
  await bridge.loadSettings();
  const first = bridge.connect();
  const cancelled = assert.rejects(first, (error) => error.code === "WEBSOCKET_DISCONNECTED");
  await Promise.resolve();
  bridge.disconnect();
  await bridge.connect();
  release(credentials());
  await cancelled;
  await Promise.resolve();
  assert.equal(sockets.length, 1);
  assert.equal(sockets[0].url, "ws://127.0.0.1:43128/");
  const staleMessage = sockets[0].onmessage;
  bridge.disconnect();
  await bridge.connect();
  staleMessage({ data: JSON.stringify({ protocol_version: 1, message_type: "authentication_response", authenticated: false }) });
  assert.equal(bridge.getStatus().authenticated, true);
  bridge.disconnect();
});
test("worker reconstruction uses a fresh bootstrap without stored token or port", async () => {
  let calls = 0;
  const api = storageApi();
  for (let index = 0; index < 2; index += 1) {
    const sockets = [];
    const bridge = new WebSocketBridge(api, { socketFactory: socketClass(sockets), bootstrap: async () => {
      calls += 1; return credentials("r1", 43127 + calls);
    } });
    await bridge.loadSettings();
    await bridge.connect();
    assert.equal(sockets[0].url, `ws://127.0.0.1:${43127 + calls}/`);
    bridge.disconnect();
  }
  assert.equal(calls, 2);
});
test("automatic failures never authenticate or open a socket from invalid bootstrap", async () => {
  const sockets = [];
  const bridge = new WebSocketBridge(storageApi(), { socketFactory: socketClass(sockets),
    bootstrap: async () => ({ ...credentials(), host: "remote" }), retryDelaysMs: [] });
  await bridge.loadSettings();
  await assert.rejects(bridge.connect(), (error) => error.code === "BOOTSTRAP_PROTOCOL_ERROR");
  assert.equal(sockets.length, 0);
  assert.equal(bridge.getStatus().authenticated, false);
});
test("pre-auth business, unsupported authentication version and deadline fail closed", async () => {
  for (const mode of ["business", "version", "silent", "reject"]) {
    const bridge = new WebSocketBridge(storageApi(), { socketFactory: socketClass([], mode), bootstrap: async () => credentials(),
      authenticationTimeoutMs: 5, retryDelaysMs: [] });
    await bridge.loadSettings();
    await assert.rejects(bridge.connect());
    assert.equal(bridge.getStatus().authenticated, false);
    assert.equal(bridge.socket, null);
    bridge.disconnect();
  }
});
test("automatic channel is selected even without manual settings and never sends business Native", async () => {
  let nativeBusiness = 0;
  const native = { bootstrap: async () => credentials(), send: async () => { nativeBusiness += 1; } };
  const bridge = new TransportBridge(storageApi(), { native, socketFactory: socketClass([]), requestTimeoutMs: 5 });
  await bridge.initialize();
  assert.equal(bridge.channel, "websocket");
  await assert.rejects(bridge.send({ protocol_version: 1, request_id: "lost", message_type: "query_status", tweet_ids: ["1"] }),
    (error) => error.code === "WEBSOCKET_REQUEST_TIMEOUT");
  assert.equal(nativeBusiness, 0);
  bridge.websocket.disconnect();
});
test("bootstrap absence retries finitely and remains diagnosable", async () => {
  let calls = 0;
  const bridge = new WebSocketBridge(storageApi(), { socketFactory: socketClass([]), bootstrap: async () => {
    calls += 1; throw Object.assign(new Error("Host missing"), { code: "NATIVE_HOST_CONNECT_FAILED", retryable: true });
  }, retryDelaysMs: [1, 1] });
  await bridge.loadSettings();
  await assert.rejects(bridge.connect());
  await new Promise((resolve) => setTimeout(resolve, 20));
  assert.equal(calls, 3);
  assert.equal(bridge.retryTimer, null);
  assert.equal(bridge.getStatus().error, "NATIVE_HOST_CONNECT_FAILED");
  bridge.disconnect();
});
test("sender allowlist blocks content management and foreign identities", () => {
  const api = { runtime: { id: "extension-id" } };
  const content = { id: "extension-id", url: "https://x.com/alice", tab: { id: 1 } };
  const page = { id: "extension-id", url: "chrome-extension://extension-id/options.html", tab: { id: 2 } };
  for (const type of ["save_websocket_settings", "get_websocket_settings", "reconnect_transport"]) {
    assert.equal(authorizedSender(api, content, { type }), false);
    assert.equal(authorizedSender(api, page, { type }), true);
  }
  assert.equal(authorizedSender(api, content, { type: "query_status", tweet_ids: ["123"] }), true);
  assert.equal(authorizedSender(api, content, { type: "query_status", tweet_ids: "123" }), false);
  assert.equal(authorizedSender(api, { ...content, url: "https://x.com.evil/" }, { type: "get_extension_status" }), false);
  assert.equal(authorizedSender(api, { ...page, id: "foreign" }, { type: "get_extension_status" }), false);
});
test("legacy settings require explicit mode and never become automatic credentials", () => {
  assert.equal(normalizeWebSocketSettings({ token: "legacy-secret" }).token, "");
  assert.equal(normalizeWebSocketSettings({ mode: "legacy", token: "legacy-secret" }).token, "legacy-secret");
  assert.equal(normalizeWebSocketSettings().mode, "automatic");
});

test("business arriving during worker initialization waits for settings", async () => {
  let release, sends = 0, response;
  const api = { runtime: { id: "extension-id", onMessage: event() } };
  const bridge = {
    initialize: () => new Promise((resolve) => { release = resolve; }),
    send: async () => { sends += 1; return { state: "QUEUED" }; },
  };
  // Capture the actual listener rather than calling the bridge directly.
  let listener;
  api.runtime.onMessage = { addListener(value) { listener = value; } };
  installBackground(api, bridge);
  assert.equal(listener({ type: "query_status", tweet_ids: ["1"] },
    { id: "extension-id", url: "https://x.com/alice", tab: { id: 1 } }, (value) => { response = value; }), true);
  await Promise.resolve();
  assert.equal(sends, 0);
  release();
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(sends, 1);
  assert.equal(response.state, "QUEUED");
});
test("post-auth protocol failure closes socket and rejects pending without exposing input", async () => {
  const sockets = [];
  const bridge = new WebSocketBridge(storageApi(), { socketFactory: socketClass(sockets), bootstrap: async () => credentials(), retryDelaysMs: [] });
  await bridge.loadSettings();
  await bridge.connect();
  const pending = bridge.send({ protocol_version: 1, request_id: "pending", message_type: "query_status", tweet_ids: ["1"] });
  await Promise.resolve();
  sockets[0].onmessage({ data: 'invalid ' + "a".repeat(64) });
  await assert.rejects(pending, (error) => error.code === "WEBSOCKET_PROTOCOL_ERROR" && !error.message.includes("a".repeat(64)));
  assert.equal(bridge.getStatus().authenticated, false);
  assert.equal(bridge.ensureConnected(), false);
});

test("pairing errors follow the shared bounded contract", () => {
  const error = { protocol_version: 1, message_type: "error", request_id: "r1",
    error_code: "DESKTOP_NOT_READY", error_message: "Desktop unavailable", retryable: true };
  assert.equal(validPairingError(error, "r1"), true);
  for (const patch of [{ protocol_version: 2 }, { retryable: "true" }, { request_id: "other" },
    { error_code: "line\nbreak" }, { error_message: "x".repeat(513) }, { ticket: "a".repeat(64) }]) {
    assert.equal(validPairingError({ ...error, ...patch }, "r1"), false);
  }
});
