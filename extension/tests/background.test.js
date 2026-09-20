import test from "node:test";
import assert from "node:assert/strict";
import {
  NativeBridge,
  createArchiveRequest,
  createQueryStatusRequest,
  isProtocolResponse,
} from "../src/background.js";

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