import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const mainSource = readFileSync(new URL("../src/main.jsx", import.meta.url), "utf8");
const uiStateSource = readFileSync(new URL("../src/lib/ui-state.js", import.meta.url), "utf8");

test("main.jsx wires the clipboard command through the Rust backend", () => {
  assert.match(mainSource, /invoke\("copy_text_to_clipboard"/);
});

test("main.jsx loads the real sidecar path instead of a static label", () => {
  assert.match(mainSource, /invoke\("get_sidecar_path"/);
  assert.match(mainSource, /CopyablePath label="Sidecar gallery-dl 路径"/);
  assert.doesNotMatch(mainSource, /便携目录 \/ sidecar \/ gallery-dl/);
});

test("main.jsx drops the aria2 multi-version picker semantics", () => {
  assert.doesNotMatch(mainSource, /list_aria2_releases/);
  assert.doesNotMatch(mainSource, /aria2Releases/);
  assert.match(mainSource, /validate_aria2_path/);
  assert.match(mainSource, /save_aria2_path/);
});

test("main.jsx routes the sidebar extension state through the explicit mapping", () => {
  assert.match(mainSource, /ExtensionConnectionStatus/);
  assert.doesNotMatch(mainSource, /loading=\{initialLoad \|\| extension\.browser_connection === "unknown"\}/);
});

test("ui-state.js keeps the extension state contract local", () => {
  assert.match(uiStateSource, /export function extensionSidebarState/);
  assert.match(uiStateSource, /文件缺失/);
});
