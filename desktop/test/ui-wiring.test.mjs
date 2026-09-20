import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const mainSource = readFileSync(new URL("../src/main.jsx", import.meta.url), "utf8");
const settingsSource = readFileSync(new URL("../src/pages/settings-page.jsx", import.meta.url), "utf8");
const uiStateSource = readFileSync(new URL("../src/lib/ui-state.js", import.meta.url), "utf8");

test("main.jsx wires the clipboard command through the Rust backend", () => {
  assert.match(mainSource, /invoke\("copy_text_to_clipboard"/);
});

test("main.jsx loads the real sidecar path instead of a static label", () => {
  assert.match(mainSource, /invoke\("get_sidecar_path"/);
  assert.match(settingsSource, /CopyablePath label="gallery-dl 可执行文件"/);
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
  assert.match(uiStateSource, /not_loaded/);
});

test("settings page exposes executable selection and the external Extension source", () => {
  assert.match(mainSource, /invoke\("save_gallery_dl_path"/);
  assert.match(mainSource, /validate_gallery_dl_path/);
  assert.match(mainSource, /@tauri-apps\/plugin-dialog/);
  assert.doesNotMatch(mainSource, /invoke\("import_extension_directory"/);
  assert.match(settingsSource, /未检测到 gallery-dl 可执行文件/);
  assert.match(settingsSource, /选择文件/);
  assert.doesNotMatch(settingsSource, /gallery-dl-path/);
  assert.doesNotMatch(settingsSource, /Core Package 外部 gallery-dl/);
  assert.match(settingsSource, /Extension/);
  assert.doesNotMatch(settingsSource, /导入本地 Extension/);
});

test("settings page keeps aria2 actions without an editable path input", () => {
  assert.match(settingsSource, /自定义 aria2 路径/);
  assert.match(settingsSource, /下载并安装/);
  assert.match(mainSource, /validate_aria2_path/);
  assert.match(mainSource, /save_aria2_path/);
  assert.doesNotMatch(settingsSource, /id="aria2-custom-path"/);
});

test("settings page exposes the Core Bootstrap status boundary", () => {
  assert.match(mainSource, /invoke\("get_component_bootstrap_status"/);
  assert.match(settingsSource, /Core Bootstrap/);
  assert.match(settingsSource, /组件目录只激活经过固定 catalog 校验的本地版本/);
});

test("dashboard and shared status layout expose the intended UI contracts", () => {
  const dashboardSource = readFileSync(new URL("../src/pages/dashboard-page.jsx", import.meta.url), "utf8");
  const sharedSource = readFileSync(new URL("../src/pages/shared.jsx", import.meta.url), "utf8");
  assert.equal((dashboardSource.match(/<MetricCard/g) || []).length, 4);
  assert.match(sharedSource, /className="status-copy"/);
  assert.doesNotMatch(sharedSource, /className="status-row"[^>]*>.*<span>\{detail\}/s);
});
