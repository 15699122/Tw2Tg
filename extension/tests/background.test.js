import test from "node:test";
import assert from "node:assert/strict";
import { NativeBridge, createArchiveRequest, createQueryStatusRequest, isProtocolResponse } from "../src/background.js";
import { WebSocketBridge } from "../src/websocket-bridge.js";

function createEvent() {
  const listeners = [];
  return { addListener(listener) { listeners.push(listener); }, emit(value) { for (const listener of listeners) listener(value); } };
}

function createChrome(port) {
  return { runtime: { connectNative: () => port } };
}

test("builds versioned browser messages without secrets or paths", () => {
  const archive = createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r1");
  assert.equal(archive.message_type, "archive_request");
  assert.equal(archive.protocol_version, 1);
  assert.equal(archive.request_id, "r1");
  assert.deepEqual(createQueryStatusRequest(["1", 1, "2"], "r2").tweet_ids, ["1", "2"]);
  assert.equal(JSON.stringify(archive).includes("cookie"), false);
  assert.equal(isProtocolResponse({ protocol_version: 1, request_id: "r1", message_type: "archive_status" }), true);
  assert.equal(isProtocolResponse({ protocol_version: 1, request_id: "r1", message_type: "archive_status_batch" }), true);
  assert.equal(isProtocolResponse({ protocol_version: 1, request_id: "r1", message_type: "unknown" }), false);
  assert.equal(isProtocolResponse({ protocol_version: 2, request_id: "r1", message_type: "archive_status" }), false);
});

test("routes Native Messaging responses to the matching request", async () => {
  const port = { onMessage: createEvent(), onDisconnect: createEvent(), postMessage(message) { setTimeout(() => this.onMessage.emit({ protocol_version: 1, request_id: message.request_id, message_type: "archive_status", state: "QUEUED" }), 0); } };
  const bridge = new NativeBridge(createChrome(port));
  await assert.doesNotReject(async () => {
    const response = await bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r1"));
    assert.equal(response.state, "QUEUED");
  });
});

test("rejects pending requests when the Native Host disconnects", async () => {
  const port = { onMessage: createEvent(), onDisconnect: createEvent(), postMessage() {} };
  const bridge = new NativeBridge(createChrome(port));
  const pending = bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r1"));
  port.onDisconnect.emit();
  await assert.rejects(pending, /Native Host disconnected/);
});

test("uses the browser runtime lastError when Native Messaging disconnects", async () => {
  const port = { onMessage: createEvent(), onDisconnect: createEvent(), postMessage() {} };
  const chrome = createChrome(port);
  chrome.runtime.lastError = { message: "Specified native messaging host not found." };
  const bridge = new NativeBridge(chrome);
  const pending = bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r-last-error"));
  port.onDisconnect.emit();
  await assert.rejects(pending, /Specified native messaging host not found/);
});

test("creates a fresh Native Messaging port after disconnect", async () => {
  const ports = [
    { onMessage: createEvent(), onDisconnect: createEvent(), postMessage() {} },
    { onMessage: createEvent(), onDisconnect: createEvent(), postMessage(message) { this.onMessage.emit({ protocol_version: 1, request_id: message.request_id, message_type: "archive_status", state: "QUEUED" }); } },
  ];
  let index = 0;
  const chrome = { runtime: { connectNative: () => ports[index++] } };
  const bridge = new NativeBridge(chrome);
  const first = bridge.connect();
  first.onDisconnect.emit();
  const response = await bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r-reconnect"));
  assert.equal(response.state, "QUEUED");
  assert.equal(index, 2);
});

test("converts synchronous connect failures into useful errors", async () => {
  const chrome = { runtime: { connectNative() { throw new Error("Specified native messaging host not found."); } } };
  const bridge = new NativeBridge(chrome);
  await assert.rejects(
    bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r-connect-failure")),
    /Specified native messaging host not found/,
  );
});

test("rejects a Native Pipe unavailable response", async () => {
  const port = {
    onMessage: createEvent(),
    onDisconnect: createEvent(),
    postMessage(message) {
      setTimeout(() => this.onMessage.emit({
        protocol_version: 1,
        request_id: message.request_id,
        message_type: "error",
        error_code: "NATIVE_PIPE_UNAVAILABLE",
        error_message: "Named Pipe forwarding is not configured",
      }), 0);
    },
  };
  const bridge = new NativeBridge(createChrome(port));
  await assert.rejects(
    bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r1")),
    /Named Pipe forwarding is not configured/,
  );
});

test("times out pending requests and removes them from the bridge", async () => {
  const port = { onMessage: createEvent(), onDisconnect: createEvent(), postMessage() {} };
  const bridge = new NativeBridge(createChrome(port), undefined, { requestTimeoutMs: 10 });
  const pending = bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r-timeout"));
  await assert.rejects(pending, (error) => {
    assert.equal(error.code, "NATIVE_REQUEST_TIMEOUT");
    assert.equal(error.retryable, true);
    return /timed out/.test(error.message);
  });
  assert.equal(bridge.pending.size, 0);
});

test("rejects duplicate request IDs without replacing the original waiter", async () => {
  const port = {
    onMessage: createEvent(),
    onDisconnect: createEvent(),
    postMessage() {},
  };
  const bridge = new NativeBridge(createChrome(port), undefined, { requestTimeoutMs: 30 });
  const first = bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r-duplicate"));
  await assert.rejects(
    bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r-duplicate")),
    (error) => error.code === "DUPLICATE_REQUEST_ID",
  );
  port.onDisconnect.emit();
  await assert.rejects(first, /Native Host disconnected/);
});

test("preserves structured Native Host errors", async () => {
  const port = {
    onMessage: createEvent(),
    onDisconnect: createEvent(),
    postMessage(message) {
      this.onMessage.emit({
        protocol_version: 1,
        request_id: message.request_id,
        message_type: "error",
        error_code: "PROTOCOL_ERROR",
        error_message: "invalid browser payload",
        retryable: false,
      });
    },
  };
  const bridge = new NativeBridge(createChrome(port));
  await assert.rejects(
    bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r-structured")),
    (error) => {
      assert.equal(error.code, "PROTOCOL_ERROR");
      assert.equal(error.retryable, false);
      assert.equal(error.request_id, "r-structured");
      return error.message === "invalid browser payload";
    },
  );
});

test("ignores late messages and disconnects from an old port generation", async () => {
  const ports = [
    { onMessage: createEvent(), onDisconnect: createEvent(), postMessage() {} },
    { onMessage: createEvent(), onDisconnect: createEvent(), postMessage(message) {
      this.onMessage.emit({ protocol_version: 1, request_id: message.request_id, message_type: "archive_status", state: "QUEUED" });
    } },
  ];
  let index = 0;
  const bridge = new NativeBridge({ runtime: { connectNative: () => ports[index++] } });
  const oldPort = bridge.connect();
  oldPort.onDisconnect.emit();
  const response = await bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r-generation"));
  assert.equal(response.state, "QUEUED");
  oldPort.onDisconnect.emit();
  oldPort.onMessage.emit({ protocol_version: 1, request_id: "r-generation", message_type: "error", error_code: "OLD", error_message: "stale" });
  assert.equal(bridge.port, ports[1]);
});

test("converts postMessage failures into structured errors and cleans pending state", async () => {
  const port = { onMessage: createEvent(), onDisconnect: createEvent(), postMessage() { throw new Error("post failed"); } };
  const bridge = new NativeBridge(createChrome(port));
  await assert.rejects(
    bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "r-post-failure")),
    (error) => error.code === "NATIVE_REQUEST_FAILED" && error.retryable === true,
  );
  assert.equal(bridge.pending.size, 0);
});

test("WebSocket bridge authenticates before routing a BrowserResponse", async () => {
  let socket;
  class FakeSocket {
    constructor(url) { this.url = url; this.readyState = 0; socket = this; queueMicrotask(() => { this.readyState = 1; this.onopen?.(); }); }
    send(value) {
      const message = JSON.parse(value);
      if (message.message_type === "authenticate") {
        queueMicrotask(() => this.onmessage?.({ data: JSON.stringify({ protocol_version: 1, message_type: "authentication_response", authenticated: true }) }));
      } else {
        queueMicrotask(() => this.onmessage?.({ data: JSON.stringify({ protocol_version: 1, request_id: message.request_id, message_type: "archive_status", state: "QUEUED" }) }));
      }
    }
    close() { this.readyState = 3; this.onclose?.(); }
  }
  const bridge = new WebSocketBridge({}, { socketFactory: FakeSocket, requestTimeoutMs: 50 });
  bridge.settings = { enabled: true, port: 17321, token: "pairing-token" };
  const response = await bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "ws-auth"));
  assert.equal(socket.url, "ws://127.0.0.1:17321");
  assert.equal(response.state, "QUEUED");
  assert.equal(bridge.getStatus().state, "connected");
});

test("WebSocket bridge rejects pending requests when the socket disconnects", async () => {
  let socket;
  class FakeSocket {
    constructor() { this.readyState = 0; socket = this; queueMicrotask(() => { this.readyState = 1; this.onopen?.(); }); }
    send(value) {
      if (JSON.parse(value).message_type === "authenticate") queueMicrotask(() => this.onmessage?.({ data: JSON.stringify({ protocol_version: 1, message_type: "authentication_response", authenticated: true }) }));
    }
    close() { this.readyState = 3; this.onclose?.(); }
  }
  const bridge = new WebSocketBridge({}, { socketFactory: FakeSocket, requestTimeoutMs: 50 });
  bridge.settings = { enabled: true, port: 17321, token: "pairing-token" };
  await bridge.connect();
  const pending = bridge.send(createArchiveRequest({ tweet_id: "1", url: "https://x.com/i/status/1" }, "ws-disconnect"));
  await Promise.resolve();
  socket.onclose();
  await assert.rejects(pending, (error) => error.code === "WEBSOCKET_DISCONNECTED");
  assert.equal(bridge.pending.size, 0);
});

test("WebSocket bridge does not connect while disabled and clears pending reconnect", async () => {
  let connections = 0;
  class FakeSocket {
    constructor() { connections += 1; this.readyState = 0; queueMicrotask(() => { this.readyState = 1; this.onopen?.(); }); }
    send(value) {
      if (JSON.parse(value).message_type === "authenticate") queueMicrotask(() => this.onmessage?.({ data: JSON.stringify({ protocol_version: 1, message_type: "authentication_response", authenticated: true }) }));
    }
    close() { this.readyState = 3; this.onclose?.(); }
  }
  const bridge = new WebSocketBridge({}, { socketFactory: FakeSocket, retryDelaysMs: [1] });
  bridge.settings = { enabled: false, port: 17321, token: "pairing-token" };
  await assert.rejects(bridge.connect(), (error) => error.code === "WEBSOCKET_DISABLED");
  assert.equal(connections, 0);
  bridge.settings.enabled = true;
  await bridge.connect();
  bridge.socket.onclose();
  assert.ok(bridge.retryTimer);
  bridge.settings.enabled = false;
  bridge.disconnect("settings changed");
  await new Promise((resolve) => setTimeout(resolve, 5));
  assert.equal(connections, 1);
  assert.equal(bridge.retryTimer, null);
  assert.equal(bridge.getStatus().state, "disabled");
});


test("WebSocket bridge times out authentication instead of leaving background ready pending", async () => {
  class SilentSocket {
    constructor() { this.readyState = 0; queueMicrotask(() => { this.readyState = 1; this.onopen?.(); }); }
    send() {}
    close() { this.readyState = 3; this.onclose?.(); }
  }
  const bridge = new WebSocketBridge({}, { socketFactory: SilentSocket, authenticationTimeoutMs: 5, retryDelaysMs: [1] });
  bridge.settings = { enabled: true, port: 17321, token: "pairing-token" };
  await assert.rejects(bridge.connect(), (error) => error.code === "WEBSOCKET_AUTH_TIMEOUT");
  assert.equal(bridge.getStatus().state, "auth_timeout");
  assert.equal(bridge.socket, null);
});

test("WebSocket bridge does not retry an authentication failure until settings change", async () => {
  let connections = 0;
  class FakeSocket {
    constructor() { connections += 1; this.readyState = 0; queueMicrotask(() => { this.readyState = 1; this.onopen?.(); }); }
    send() { queueMicrotask(() => this.onmessage?.({ data: JSON.stringify({ protocol_version: 1, message_type: "authentication_response", authenticated: false, error_code: "BAD_TOKEN" }) })); }
    close() { this.readyState = 3; }
  }
  const bridge = new WebSocketBridge({}, { socketFactory: FakeSocket, retryDelaysMs: [1] });
  bridge.settings = { enabled: true, port: 17321, token: "wrong-token" };
  await assert.rejects(bridge.connect(), (error) => error.code === "WEBSOCKET_AUTH_FAILED");
  await new Promise((resolve) => setTimeout(resolve, 5));
  assert.equal(connections, 1);
  assert.equal(bridge.retryTimer, null);
  assert.equal(bridge.getStatus().state, "auth_failed");
});
