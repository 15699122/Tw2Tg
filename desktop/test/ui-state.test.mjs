import test from "node:test";
import assert from "node:assert/strict";
import { displayFileName, extensionSidebarState, aria2StatusText } from "../src/lib/ui-state.js";

test("displayFileName extracts the file name from windows-style paths", () => {
  assert.equal(displayFileName("C:\\tools\\aria2\\aria2c.exe"), "aria2c.exe");
});

test("displayFileName extracts the file name from posix-style paths", () => {
  assert.equal(displayFileName("/home/user/sidecar/gallery-dl/gallery-dl"), "gallery-dl");
});

test("displayFileName keeps a bare file name unchanged", () => {
  assert.equal(displayFileName("gallery-dl.exe"), "gallery-dl.exe");
});

test("displayFileName reports a missing path", () => {
  assert.equal(displayFileName(""), "未检测到路径");
  assert.equal(displayFileName(null), "未检测到路径");
});

test("extensionSidebarState shows checking during initial load", () => {
  assert.deepEqual(
    extensionSidebarState({ filesReady: false, browserConnection: "unknown", initialLoad: true }),
    { tone: "muted", text: "检测中…" },
  );
});

test("extensionSidebarState shows connected only for an explicit connected state", () => {
  assert.deepEqual(
    extensionSidebarState({ filesReady: true, browserConnection: "connected", initialLoad: false }),
    { tone: "online", text: "已连接" },
  );
});

test("extensionSidebarState shows missing files instead of a stuck checking state", () => {
  assert.deepEqual(
    extensionSidebarState({ filesReady: false, browserConnection: "unknown", initialLoad: false }),
    { tone: "error", text: "文件缺失" },
  );
});

test("extensionSidebarState falls back to not connected when files are ready", () => {
  assert.deepEqual(
    extensionSidebarState({ filesReady: true, browserConnection: "disconnected", initialLoad: false }),
    { tone: "error", text: "未连接" },
  );
});

test("aria2StatusText prefers the detected version", () => {
  assert.equal(
    aria2StatusText({ found: true, version: "1.37.0", path: null, source: null, error: null }),
    "v1.37.0",
  );
});

test("aria2StatusText explains a missing installation", () => {
  assert.equal(aria2StatusText(null), "当前程序未找到 aria2c");
  assert.equal(
    aria2StatusText({ found: false, version: null, path: null, source: null, error: null }),
    "当前程序未找到 aria2c",
  );
});
