import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const mainSource = readFileSync(new URL("../src/main.jsx", import.meta.url), "utf8");
const settingsSource = readFileSync(new URL("../src/pages/settings-page.jsx", import.meta.url), "utf8");
const uiStateSource = readFileSync(new URL("../src/lib/ui-state.js", import.meta.url), "utf8");
const bootstrapSource = readFileSync(new URL("../src/bootstrap.js", import.meta.url), "utf8");
const indexSource = readFileSync(new URL("../index.html", import.meta.url), "utf8");
const releaseWorkflowSource = readFileSync(new URL("../../.github/workflows/windows-release.yml", import.meta.url), "utf8");
const wdioConfigSource = readFileSync(new URL("../wdio.conf.mjs", import.meta.url), "utf8");

test("frontend bootstrap reports startup stages and uncaught failures", () => {
  assert.match(bootstrapSource, /installFrontendBootstrap/);
  assert.match(bootstrapSource, /unhandledrejection/);
  assert.match(bootstrapSource, /startup_timeout/);
  assert.match(mainSource, /react_mount_started/);
  assert.match(mainSource, /initial_ipc_settled/);
  assert.doesNotMatch(mainSource, /setStartupState\("initial_ipc_(started|settled)"\)/);
});

test("index.html keeps a non-React startup fallback", () => {
  assert.match(indexSource, /id="startup-fallback"/);
  assert.match(indexSource, /正在启动 XArchive/);
  assert.match(indexSource, /前端资源加载失败/);
  assert.match(indexSource, /启动超时/);
});

test("frontend diagnostics use the restricted Rust logging command", () => {
  assert.match(bootstrapSource, /log_frontend_event/);
  assert.match(readFileSync(new URL("../src-tauri/src/commands.rs", import.meta.url), "utf8"), /FrontendDiagnosticEvent/);
});

test("native smoke collects startup evidence before asserting dashboard content", () => {
  const smokeSource = readFileSync(new URL("../e2e/specs/dashboard.e2e.mjs", import.meta.url), "utf8");
  const supportSource = readFileSync(new URL("../e2e/support/native-startup.mjs", import.meta.url), "utf8");
  assert.match(smokeSource, /react_mount_completed/);
  assert.match(supportSource, /document\.readyState/);
  assert.match(supportSource, /xarchiveStartup/);
  assert.match(supportSource, /startup-failure\.png/);
  assert.match(smokeSource, /captureReadinessFailure/);
  // WQ-P0-WHITE-04C: session 建立瞬间的初始 target 快照必须在等待应用
  // 文档之前写入诊断目录，否则 target-attachment 诊断缺少第一份证据。
  assert.match(smokeSource, /snapshotSessionStart/);
  assert.match(supportSource, /session-start\.json/);
  assert.match(supportSource, /browser\.capabilities/);
});

test("WDIO Windows driver configuration is explicit and opt-in for downloads", () => {
  assert.match(wdioConfigSource, /WDIO_AUTO_INSTALL_TAURI_DRIVER === "1"/);
  assert.match(wdioConfigSource, /WDIO_AUTO_DOWNLOAD_EDGE_DRIVER === "1"/);
  assert.match(wdioConfigSource, /EDGEDRIVER_VERSION/);
  assert.match(wdioConfigSource, /edgeDriverVersion/);
  assert.match(wdioConfigSource, /TAURI_DRIVER_PATH/);
  assert.match(wdioConfigSource, /tauriDriverPath/);
  assert.match(wdioConfigSource, /WEBVIEW2_BROWSER_EXECUTABLE_FOLDER/);
  assert.match(wdioConfigSource, /env: \{ WEBVIEW2_BROWSER_EXECUTABLE_FOLDER: webview2RuntimePath \}/);
  // WQ-P0-WHITE-04B: @wdio/tauri-service 的日志捕获读取 WDIO config 的
  // outputDir（service 选项 logDir 只在 standalone 路径生效），必须显式
  // 指向 WDIO_LOG_DIR，否则日志落到 desktop/logs。
  assert.match(wdioConfigSource, /outputDir: logDir/);
});

test("Windows release gate preserves preflight diagnostics and blocks upload on failure", () => {
  assert.match(releaseWorkflowSource, /Windows UI readiness preflight/);
  assert.match(releaseWorkflowSource, /windows-ui-readiness-preflight\.ps1/);
  assert.match(releaseWorkflowSource, /if: always\(\)/);
  assert.match(releaseWorkflowSource, /Final executable UI readiness gate failed/);
  assert.match(releaseWorkflowSource, /Create application-only 7z archive/);
  assert.match(releaseWorkflowSource, /webdriver-tools/);
  // WQ-P0-WHITE-01D: 隔离检查对应用/driver 残留必须 FAIL 并写入诊断，
  // msedgewebview2 后台活动只作为诊断信息。
  assert.match(releaseWorkflowSource, /isolation-failure\.txt/);
  assert.match(releaseWorkflowSource, /residual app\/driver processes before gate/);
  // WQ-P0-WHITE-04B: gate 通过但没有任何非空 *.log 时同样判 FAIL，
  // 不再依赖 Copy-Item -ErrorAction SilentlyContinue 掩盖日志缺失。
  assert.match(releaseWorkflowSource, /log-capture-status\.txt/);
  assert.match(releaseWorkflowSource, /WDIO log capture contract violated/);
});

test("the WDIO service banner fix is wired for clean installs", () => {
  const rootPackageSource = readFileSync(new URL("../../package.json", import.meta.url), "utf8");
  const desktopPackageSource = readFileSync(new URL("../package.json", import.meta.url), "utf8");
  assert.match(rootPackageSource, /"postinstall": "node desktop\/scripts\/patch-wdio-tauri-service\.mjs"/);
  assert.match(desktopPackageSource, /"pretest:e2e": "node scripts\/patch-wdio-tauri-service\.mjs"/);
  assert.match(desktopPackageSource, /"pretest:e2e:windows": "node scripts\/patch-wdio-tauri-service\.mjs"/);
  assert.match(desktopPackageSource, /"pretest:e2e:windows:advanced": "node scripts\/patch-wdio-tauri-service\.mjs"/);
  const patchScriptSource = readFileSync(new URL("../scripts/patch-wdio-tauri-service.mjs", import.meta.url), "utf8");
  assert.match(patchScriptSource, /\(\?:MSEdgeDriver\|Microsoft Edge WebDriver\)/);
  assert.match(patchScriptSource, /already-patched/);
});

test("Windows readiness preflight records the Edge driver banner compatibility", () => {
  const preflightSource = readFileSync(new URL("../scripts/windows-ui-readiness-preflight.ps1", import.meta.url), "utf8");
  assert.match(preflightSource, /msedgedriver_banner_accepted/);
  assert.match(preflightSource, /Microsoft Edge WebDriver/);
  assert.match(preflightSource, /MSEdgeDriver/);
  assert.match(preflightSource, /@wdio\/tauri-service 1\.4\.0/);
});

test("Linux Edge banner helper keeps the service limitation explicit", () => {
  const helperSource = readFileSync(new URL("../scripts/edge-driver-banner.mjs", import.meta.url), "utf8");
  assert.match(helperSource, /WDIO_TAURI_SERVICE_VERSION = "1\.4\.0"/);
  assert.match(helperSource, /MSEdgeDriver \(\[\\d\.\]\+\)/);
  assert.match(helperSource, /Microsoft Edge WebDriver/);
  assert.match(helperSource, /test infrastructure only/);
});

test("account batch page wires durable controls", () => {
  const batchesSource = readFileSync(new URL("../src/pages/batches-page.jsx", import.meta.url), "utf8");
  assert.match(mainSource, /BatchesPage/);
  assert.match(mainSource, /invoke\("create_account_batch"/);
  assert.match(mainSource, /invoke\(command, \{ batchId \}\)/);
  assert.match(mainSource, /window\.setInterval/);
  assert.match(mainSource, /invoke\("list_jobs", \{ limit: 20 \}\)/);
  assert.match(mainSource, /invoke\("list_account_batches", \{ limit: 20 \}\)/);
  assert.match(mainSource, /window\.clearInterval/);
  assert.match(batchesSource, /pause_account_batch/);
  assert.match(batchesSource, /resume_account_batch/);
  assert.match(batchesSource, /cancel_account_batch/);
  assert.match(batchesSource, /retry_account_batch/);
  assert.match(batchesSource, /不显示百分比/);
});

test("failed jobs expose their durable error code and message", () => {
  const sharedSource = readFileSync(new URL("../src/pages/shared.jsx", import.meta.url), "utf8");
  assert.match(sharedSource, /job\.last_error_code/);
  assert.match(sharedSource, /job\.last_error_message/);
});

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
  assert.match(
    mainSource,
    /function Sidebar\(\{[^}]*extension, extensionBusy, initialLoad[^}]*\}\)/s,
  );
  assert.match(mainSource, /<ExtensionConnectionStatus[^>]*checking=\{extensionBusy\}/);
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
