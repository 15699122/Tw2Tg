import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const mainSource = readFileSync(new URL("../src/main.jsx", import.meta.url), "utf8");
const settingsSource = readFileSync(new URL("../src/pages/settings-page.jsx", import.meta.url), "utf8");
const downloadsSource = readFileSync(new URL("../src/pages/download-settings-page.jsx", import.meta.url), "utf8");
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

test("WDIO Windows driver configuration is explicit and overridable", () => {
  // Driver acquisition stays opt-out (`=0` disables) so a validation run can
  // pin a prepared driver, while the release workflow passes the explicit
  // opt-out for both switches. See docs/development/setup.md.
  assert.match(wdioConfigSource, /WDIO_AUTO_INSTALL_TAURI_DRIVER !== "0"/);
  assert.match(wdioConfigSource, /WDIO_AUTO_DOWNLOAD_EDGE_DRIVER !== "0"/);
  assert.match(releaseWorkflowSource, /WDIO_AUTO_INSTALL_TAURI_DRIVER: "0"/);
  assert.match(releaseWorkflowSource, /WDIO_AUTO_DOWNLOAD_EDGE_DRIVER: "0"/);
  assert.match(wdioConfigSource, /EDGEDRIVER_VERSION/);
  assert.match(wdioConfigSource, /edgeDriverVersion/);
  assert.match(wdioConfigSource, /TAURI_DRIVER_PATH/);
  assert.match(wdioConfigSource, /tauriDriverPath/);
  assert.match(wdioConfigSource, /EDGEDRIVER_PATH/);
  assert.match(wdioConfigSource, /nativeDriverPath/);
  assert.match(wdioConfigSource, /WEBVIEW2_BROWSER_EXECUTABLE_FOLDER/);
  assert.match(wdioConfigSource, /env: \{ WEBVIEW2_BROWSER_EXECUTABLE_FOLDER: fixedRuntimeFolder \}/);
  // WQ-ENG-09b-ORD reviewed recipe: direct msedgedriver mode skips the
  // tauri-service and addresses the recipe-started driver itself.
  assert.match(wdioConfigSource, /WDIO_DIRECT_DRIVER === "1"/);
  assert.match(wdioConfigSource, /services: directDriver \? \[\]/);
  // WQ-P0-WHITE-04B: @wdio/tauri-service 的日志捕获读取 WDIO config 的
  // outputDir（service 选项 logDir 只在 standalone 路径生效），必须显式
  // 指向 WDIO_LOG_DIR，否则日志落到 desktop/logs。
  assert.match(wdioConfigSource, /outputDir: logDir/);
});

test("independent Windows WDIO preserves preflight and failure diagnostics", () => {
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

test("release asset integrity gates upload, while independent WDIO consumes this run's executable", () => {
  const buildJob = releaseWorkflowSource.match(/^  build-windows:\n[\s\S]*?(?=^  publish-release:)/m)?.[0];
  const publishJob = releaseWorkflowSource.match(/^  publish-release:\n[\s\S]*?(?=^  validate-wdio:)/m)?.[0];
  const [_, validate] = releaseWorkflowSource.split(/^  validate-wdio:\s*$/m);
  assert.ok(buildJob && publishJob, "build and publication must remain separate jobs");
  assert.ok(validate);
  assert.match(buildJob, /Verify complete release assets before upload/);
  assert.match(buildJob, /Release asset\/manifest mismatch/);
  assert.match(buildJob, /SHA256SUMS mismatch/);
  assert.ok(buildJob.indexOf("Verify complete release assets before upload") < buildJob.indexOf("Stage verified publication inventory"));
  assert.match(releaseWorkflowSource, /^permissions:\n  contents: read/m);
  assert.doesNotMatch(buildJob, /gh release (create|upload)/);
  assert.match(publishJob, /needs: build-windows/);
  assert.match(publishJob, /if: needs\.build-windows\.result == 'success'/);
  assert.match(publishJob, /permissions:\n\s+contents: write/);
  assert.match(publishJob, /Verify publication identity and all digests/);
  assert.match(publishJob, /Publish without overwriting existing assets/);
  assert.match(publishJob, /Release asset already exists/);
  assert.doesNotMatch(publishJob, /--clobber/);
  assert.doesNotMatch(buildJob, /npm run test:e2e:windows/);
  assert.match(validate, /needs: build-windows/);
  assert.match(validate, /needs\.build-windows\.result == 'success'/);
  assert.match(validate, /Set diagnostic directory/);
  assert.doesNotMatch(validate.split(/^    steps:/m)[0], /runner\.temp/);
  assert.match(validate, /actions\/download-artifact@v4/);
  assert.match(validate, /wdio-input-\$\{\{ github\.run_id \}\}/);
  assert.match(validate, /identity\.source_sha -ne \$source/);
  assert.match(validate, /Downloaded executable SHA-256 mismatch/);
  assert.match(validate, /WDIO_APP_BINARY=\$exe/);
  assert.match(validate, /if: always\(\)/);
  assert.match(validate, /if-no-files-found: error/);
  assert.match(validate, /status = if .*'PASS'.*'FAIL'.*'BLOCKED'.*'NOT_RUN'/);
  assert.match(validate, /result\.json/);
  assert.match(validate, /diagnostics_upload\.outcome/);
});

test("pre-release explicitly dispatches the tagged Windows build", () => {
  const preRelease = readFileSync(new URL("../../.github/workflows/pre-release.yml", import.meta.url), "utf8");
  assert.match(preRelease, /gh release create "\$RELEASE_TAG"/);
  assert.match(preRelease, /gh workflow run windows-release\.yml --ref "\$RELEASE_TAG"/);
});

test("the release gate uses the direct msedgedriver recipe and an executable driver path", () => {
  // WQ-ENG-09b-ORD reviewed recipe: tauri-driver never forwards
  // `tauri:options.application` to webdriver 9.x, so the hosted gate must start
  // the pinned msedgedriver itself and connect directly. The v0.2.0-pre.12
  // rehearsal (run 36557205185) failed with a blank application document for
  // exactly this reason.
  assert.match(releaseWorkflowSource, /WDIO_DIRECT_DRIVER: "1"/);
  assert.match(releaseWorkflowSource, /WDIO_DIRECT_DRIVER_PORT: "4445"/);
  assert.match(releaseWorkflowSource, /--port=\$env:WDIO_DIRECT_DRIVER_PORT/);
  assert.match(releaseWorkflowSource, /\[direct-driver\] msedgedriver pid=/);
  // The toolchain step must export the executable: the service spawns this
  // value, and a directory produced `spawn ...\\tauri-driver\\bin ENOENT` in the
  // v0.2.0-pre.11 rehearsal (run 36553596970).
  assert.match(
    releaseWorkflowSource,
    /TAURI_DRIVER_PATH=\$\(Join-Path \$tauriPath 'tauri-driver\.exe'\)/,
  );
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

test("downloads page keeps aria2 actions without an editable path input", () => {
  assert.match(downloadsSource, /自定义 aria2 路径/);
  assert.match(downloadsSource, /下载并安装/);
  assert.match(mainSource, /validate_aria2_path/);
  assert.match(mainSource, /save_aria2_path/);
  assert.doesNotMatch(downloadsSource, /id="aria2-custom-path"/);
});

test("settings page exposes the Core Bootstrap status boundary", () => {
  assert.match(mainSource, /invoke\("get_component_bootstrap_status"/);
  assert.match(settingsSource, /Core Bootstrap/);
  assert.match(settingsSource, /固定 catalog 校验的本地组件版本/);
});

test("dashboard and shared status layout expose the intended UI contracts", () => {
  const dashboardSource = readFileSync(new URL("../src/pages/dashboard-page.jsx", import.meta.url), "utf8");
  const sharedSource = readFileSync(new URL("../src/pages/shared.jsx", import.meta.url), "utf8");
  assert.equal((dashboardSource.match(/<MetricCard/g) || []).length, 4);
  assert.match(sharedSource, /className="status-copy"/);
  assert.doesNotMatch(sharedSource, /className="status-row"[^>]*>.*<span>\{detail\}/s);
});

test("dashboard cards in the same row stretch to a shared bottom edge", () => {
  // `align-items: start` left the 最近任务 and 运行环境 cards at different
  // heights, so their bottom borders did not line up.
  const styleSource = readFileSync(new URL("../src/style.css", import.meta.url), "utf8");
  const gridRule = styleSource.match(/\.dashboard-grid \{[^}]*\}/)?.[0] ?? "";
  assert.match(gridRule, /align-items: stretch/);
  assert.doesNotMatch(gridRule, /align-items: start/);
  assert.match(styleSource, /\.dashboard-grid > \.ui-card \{ display: flex; flex-direction: column; \}/);
  assert.match(styleSource, /\.jobs-content \{ display: flex; flex-direction: column;/);
});

test("sidebar service status rows drop the default button border", () => {
  // The rows are native <button> elements; without an appearance reset the
  // browser painted a black border that clashed with the design language.
  const styleSource = readFileSync(new URL("../src/style.css", import.meta.url), "utf8");
  const rowRule = styleSource.match(/\.connection-line-button \{[^}]*\}/)?.[0] ?? "";
  assert.match(rowRule, /border: 0/);
  assert.match(rowRule, /background: transparent/);
  assert.match(rowRule, /width: calc\(100% - 16px\)/);
  assert.match(styleSource, /\.connection-line-button:hover \{[^}]*background: #e9e9e9/);
  // Removing the border must not remove the keyboard focus indicator.
  assert.match(styleSource, /button:focus-visible[^{]*\{ outline: 2px solid var\(--focus-ring\)/);
});

test("sidebar version label keeps balanced vertical spacing", () => {
  // The negative top margin pulled the version line onto the separator while
  // the sidebar bottom padding stayed large.
  const styleSource = readFileSync(new URL("../src/style.css", import.meta.url), "utf8");
  const versionRule = styleSource.match(/\.version-label \{[^}]*\}/)?.[0] ?? "";
  assert.doesNotMatch(versionRule, /margin: -\d/);
  assert.match(versionRule, /margin: 0 8px/);
  const separatorRule = styleSource.match(/\.sidebar-footer-separator \{[^}]*\}/)?.[0] ?? "";
  assert.match(separatorRule, /margin-block: 8px 12px/);
});


import { displayLogLevel, effectiveLevelChanged, filterLogEntries, mergeLogLines, parseLogEntries } from "../src/lib/log-lines.js";

// WQ-LOGS-020-02: the Logs page hardcoded `info`, so an optimized pre-release
// whose effective level is `debug` showed no module diagnostics even though the
// backend had written them. The display filter now follows the backend.
test("the logs display filter follows the backend effective level", () => {
  assert.equal(displayLogLevel("debug"), "debug");
  assert.equal(displayLogLevel("info"), "info");
  assert.equal(displayLogLevel("silent"), "silent");
  // A level the backend cannot report falls back to a neutral guess rather than
  // leaving the select outside its own option list.
  assert.equal(displayLogLevel(""), "info");
  assert.equal(displayLogLevel(undefined), "info");
  assert.equal(displayLogLevel("nonsense"), "info");
});

test("re-syncing the display filter only happens when the backend level changes", () => {
  // Polling reports the same level every second; treating that as a change would
  // wipe out a level the user picked on the page itself.
  assert.equal(effectiveLevelChanged("debug", "debug"), false);
  // A backend that spells the same level differently must not look like a change.
  assert.equal(effectiveLevelChanged("warning", "warn"), false);
  assert.equal(effectiveLevelChanged("debug", "info"), true);
  assert.equal(effectiveLevelChanged("info", "warn"), true);
  assert.equal(effectiveLevelChanged("", "debug"), true);
  // WQ-LOGS-020-02 must not regress: the page state and its initialisation both
  // have to come from the backend value rather than a literal `info`.
  const logsPage = readFileSync(new URL("../src/pages/logs-page.jsx", import.meta.url), "utf8");
  assert.doesNotMatch(logsPage, /useState\("info"\)/);
  assert.match(logsPage, /useState\(\(\) => displayLogLevel\(loggingLevel\)\)/);
  assert.match(logsPage, /LogsPage\(\{ loggingLevel \}\)/);
  assert.match(mainSource, /<LogsPage loggingLevel=\{status\.logging_level\} \/>/);
});

test("a debug filter actually shows the module diagnostics a pre-release writes", () => {
  const entries = parseLogEntries([
    "2026-10-01T09:00:00Z debug runtime: application runtime initialized (effective_level=debug)",
    "2026-10-01T09:00:01Z debug executor: progress",
    "2026-10-01T09:00:02Z info runtime: application ready",
  ]);
  assert.equal(filterLogEntries(entries, displayLogLevel("debug")).length, 3);
  assert.equal(filterLogEntries(entries, displayLogLevel("info")).length, 1);
});

test("logs panel keeps deliberate spacing from the page description", () => {
  // The description butted against the panel; the fix is page-scoped so
  // Dashboard and Settings spacing must stay untouched.
  const css = readFileSync(new URL("../src/style.css", import.meta.url), "utf8");
  assert.match(css, /\.logs-page-panel \{ margin-top: 24px; \}/);
  const logsPage = readFileSync(new URL("../src/pages/logs-page.jsx", import.meta.url), "utf8");
  assert.match(logsPage, /className="logs-panel logs-page-panel"/);
});

test("the auto-follow checkbox uses the project theme instead of the blue default", () => {
  const css = readFileSync(new URL("../src/style.css", import.meta.url), "utf8");
  const checked = css.match(/\.logs-follow input\[type="checkbox"\]:checked \{[^}]*\}/)?.[0] ?? "";
  assert.match(checked, /border-color: var\(--success\)/);
  assert.match(checked, /background-color: var\(--success\)/);
  // The toolbar input rule must not be allowed to give the checkbox its
  // text-field padding and height.
  const base = css.match(/\.logs-follow input\[type="checkbox"\] \{[^}]*\}/)?.[0] ?? "";
  assert.match(base, /appearance: none/);
  assert.match(base, /padding: 0/);
  assert.match(css, /\.logs-follow input\[type="checkbox"\]:focus-visible \{[^}]*outline: 2px solid var\(--focus-ring\)/);
});

test("settings spacing and monochrome icon retain release fixes", () => {
  const css = readFileSync(new URL("../src/style.css", import.meta.url), "utf8");
  assert.match(css, /\.settings-section \{[^}]*border-top: 1px solid var\(--border\)/);
  // All collapsible panels, including Core Bootstrap, use the same card border.
  assert.match(css, /\.settings-panel \{[^}]*border: 1px solid var\(--border\)/);
  assert.doesNotMatch(css, /#bootstrap-settings\s*\{/);
  assert.match(css, /\.copyable-path-text \{[^}]*gap: 4px/);
  assert.match(css, /\.aria2-icon \{[^}]*color: var\(--foreground-muted\)/);
  assert.match(css, /\.aria2-section-note \{ margin-top: 18px;/);
  assert.match(downloadsSource, /aria2-help aria2-section-note/);
  assert.match(css, /\.extension-websocket-status > div \{[^}]*align-content: start/);
  assert.match(css, /\.extension-actions \{[^}]*gap: 10px/);
  assert.match(css, /\.extension-actions \.ui-button \{ height: 34px; min-height: 34px;/);
});

test("archive directory chooser persists through the registered backend command", () => {
  const commands = readFileSync(new URL("../src-tauri/src/commands.rs", import.meta.url), "utf8");
  const entry = readFileSync(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");
  assert.match(mainSource, /const chooseArchiveDirectory = async/);
  assert.match(mainSource, /await open\(\{ multiple: false, directory: true \}\)/);
  assert.match(mainSource, /await invoke\("set_archive_directory", \{ directory: path \}\)/);
  assert.match(mainSource, /finally \{ setArchiveBusy\(false\)/);
  assert.match(settingsSource, /onClick=\{chooseArchiveDirectory\}/);
  assert.match(settingsSource, /不会移动已有文件/);
  assert.match(commands, /pub\(crate\) fn set_archive_directory/);
  assert.match(entry, /set_archive_directory,/);
});

test("sidebar service status block keeps its rows tight", () => {
  // The three status rows read as loosely spaced at the DPI used in the report.
  // Only the sidebar service block may be tightened; navigation rows are unchanged.
  const css = readFileSync(new URL("../src/style.css", import.meta.url), "utf8");
  const caption = css.match(/\.sidebar-caption \{[^}]*\}/)?.[0] ?? "";
  const row = css.match(/\.connection-line-button \{[^}]*\}/)?.[0] ?? "";
  assert.match(caption, /margin: 0 8px 6px/);
  assert.match(row, /min-height: 28px/);
  assert.match(row, /margin: 0 8px/);
  assert.match(row, /padding: 4px 8px/);
  // Tightening must not collapse the row affordances or the focus ring.
  assert.match(row, /width: calc\(100% - 16px\)/);
  assert.match(row, /border: 0/);
  assert.match(row, /background: transparent/);
  assert.match(css, /\.connection-line-button:hover \{[^}]*background: #e9e9e9/);
  assert.match(css, /button:focus-visible[^{]*\{ outline: 2px solid var\(--focus-ring\)/);
  // The main navigation row keeps its own taller touch target.
  const navItem = css.match(/\.nav-item \{[^}]*\}/)?.[0] ?? "";
  assert.match(navItem, /min-height: 40px/);
});

test("runtime panel keeps a constant gap above the settings link", () => {
  // `margin-top: auto` pushed 管理组件与设置 to the bottom of the stretched card,
  // so the gap above it grew with the job list instead of staying constant.
  const css = readFileSync(new URL("../src/style.css", import.meta.url), "utf8");
  const link = css.match(/\.dashboard-grid \.control-panel-content \.settings-link \{[^}]*\}/)?.[0] ?? "";
  assert.doesNotMatch(link, /margin-top: auto/);
  assert.match(link, /margin-top: 0/);
  // The vertical rhythm now comes from the flex gap on the panel content.
  const panel = css.match(/\.control-panel-content \{[^}]*\}/)?.[0] ?? "";
  assert.match(panel, /gap: 16px/);
  // The cards stay stretched, so the slack falls below the action group.
  assert.match(css, /\.dashboard-grid \{[^}]*align-items: stretch/);
});

test("core bootstrap panel uses shared card spacing and border", () => {
  const css = readFileSync(new URL("../src/style.css", import.meta.url), "utf8");
  assert.doesNotMatch(css, /#bootstrap-settings\s*\{/);
  assert.match(css, /\.settings-section:not\(\.settings-panel\):first-child/);
  assert.match(css, /\.settings-panel \{[^}]*border: 1px solid var\(--border\)/);
  assert.match(css, /\.settings-section \{[^}]*border-top: 1px solid var\(--border\)/);
  assert.match(css, /\.settings-section \{[^}]*padding: 24px 0/);
});

test("settings sections render in order with proxy last and downloads split out", () => {
  // DOM order, visual order and Tab order must agree. Proxy moves into the
  // trailing settings container after 存储位置 and 日志设置.
  // Only the rendered page body is inspected: the section helpers below it repeat
  // their ids in their own definitions.
  const pageBody = settingsSource.slice(
    settingsSource.indexOf("export default function SettingsPage("),
    settingsSource.indexOf("function ProxySettings("),
  );
  // aria2 and the download mode moved to their own page, so they are no longer
  // part of this list; that separation is asserted after the order check.
  // ProxySettings is a component, so its id is inside its own definition rather
  // than in the page body; its position is asserted by the index check below.
  const order = [...pageBody.matchAll(/id="(bootstrap|sidecar|extension|storage|logging)-settings"/g)]
    .map((m) => m[1]);
  assert.deepEqual(order, ["bootstrap", "sidecar", "extension", "storage", "logging"]);
  assert.match(pageBody, /<TelegramSettings expanded=\{expandedSections\.telegram \?\? false\}/);
  // Moved, not duplicated: these panels exist only on the downloads page.
  assert.doesNotMatch(settingsSource, /Aria2Settings/);
  assert.doesNotMatch(settingsSource, /TransferBackendSettings/);
  assert.match(downloadsSource, /<Aria2Settings\s/);
  assert.match(downloadsSource, /expanded=\{expandedSections\.aria2 \?\? false\}/);
  // Moved, not duplicated: exactly one ProxySettings render remains.
  assert.equal((settingsSource.match(/<ProxySettings /g) ?? []).length, 1);
  assert.match(settingsSource, /function ProxySettings\(\{[^}]*expanded, onToggle[^}]*\}\)/);
  assert.match(settingsSource, /<SettingsSection id="proxy-settings"[^>]*expanded=\{expanded\} onToggle=\{onToggle\}/);
  const proxyIndex = pageBody.indexOf("<ProxySettings ");
  const containerIndex = pageBody.indexOf("settings-layout-secondary");
  assert.ok(proxyIndex > containerIndex, "ProxySettings renders inside the trailing settings container");
});

test("downloads navigation selects the dedicated page and forwards all download controls", () => {
  assert.match(mainSource, /import DownloadSettingsPage from "\.\/pages\/download-settings-page\.jsx"/);
  assert.match(mainSource, /page === "downloads"\s*\?\s*\(\s*<DownloadSettingsPage/);
  assert.match(mainSource, /<NavItem icon="download" label="内容下载" active=\{page === "downloads"\} onClick=\{\(\) => setPage\("downloads"\)\}/);
  for (const prop of [
    "isWindows", "useAria2", "useAria2Busy", "useAria2Message", "saveUseAria2",
    "aria2", "aria2Busy", "aria2CustomPath", "aria2PathBusy", "aria2PathMessage",
    "refreshAria2", "downloadAria2", "checkAria2Path", "saveAria2Path", "chooseAria2",
    "errors", "copyPath", "copied", "expandedSections", "toggleSettingsSection",
  ]) {
    assert.match(mainSource, new RegExp(`<DownloadSettingsPage[\\s\\S]*?${prop}=`), `${prop} is wired to DownloadSettingsPage`);
  }
  assert.match(downloadsSource, /checked=\{useAria2\} disabled=\{useAria2Busy\} onCheckedChange=\{saveUseAria2\}/);
});

test("settings sections use accessible disclosure buttons and preserve service navigation", () => {
  const component = readFileSync(new URL("../src/components/settings-section.jsx", import.meta.url), "utf8");
  assert.match(component, /aria-expanded=\{expandedState\}/);
  assert.match(component, /aria-controls=\{contentId\}/);
  assert.match(component, /hidden=\{!expandedState\}/);
  assert.match(component, /settings-panel-toggle/);
  assert.match(mainSource, /const openSettingsSection = \(key\) => \{/);
  assert.match(mainSource, /setExpandedSettings\(\(current\) => \(\{ \.\.\.current, \[key\]: true \}\)\)/);
  assert.match(mainSource, /section\.querySelector\("\.settings-panel-toggle"\) \|\| section\)\.focus\(\)/);
  assert.match(mainSource, /openSettingsSection\("storage"\)/);
  assert.match(mainSource, /openSettingsSection\("sidecar"\)/);
  assert.match(mainSource, /openSettingsSection\("extension"\)/);
});

test("settings panels use concise purpose descriptions and preserve detailed help", () => {
  const telegram = readFileSync(new URL("../src/components/telegram-settings.jsx", import.meta.url), "utf8");
  const descriptions = [
    settingsSource.match(/title="Core Bootstrap" description="([^"]+)"/)?.[1],
    settingsSource.match(/title="Sidecar 配置" description="([^"]+)"/)?.[1],
    settingsSource.match(/title="浏览器 Extension" description="([^"]+)"/)?.[1],
    settingsSource.match(/title="存储位置" description="([^"]+)"/)?.[1],
    settingsSource.match(/title="日志设置" description="([^"]+)"/)?.[1],
    settingsSource.match(/title="网络代理" description="([^"]+)"/)?.[1],
    downloadsSource.match(/title="aria2" description="([^"]+)"/)?.[1],
    telegram.match(/title="Telegram"\s+description="([^"]+)"/)?.[1],
  ];
  assert.equal(descriptions.length, 8);
  assert.ok(descriptions.every((description) => description && !/JSONL|stdout|stderr|catalog|Package|不代表|SHA-256|staging|Windows x64/.test(description)));
  assert.match(settingsSource, /JSONL（每行一个 JSON 对象）格式的 v2 命令与事件/);
  assert.match(settingsSource, /固定 catalog 校验/);
  assert.match(settingsSource, /更改目录仅影响后续归档，不会移动已有文件/);
  assert.match(settingsSource, /Full Package 会预置 Extension/);
  assert.match(telegram, /Bot API 确认发送成功不代表对方已接收或已读/);
  assert.match(downloadsSource, /官方 aria2 Windows x64 发布包，下载后会校验 SHA-256/);
});

test("settings panel styles avoid legacy first-panel spacing and Telegram color overrides", () => {
  const styles = readFileSync(new URL("../src/style.css", import.meta.url), "utf8");
  assert.match(styles, /\.settings-section:not\(\.settings-panel\):first-child/);
  assert.doesNotMatch(styles, /#bootstrap-settings\s*\{/);
  assert.doesNotMatch(styles, /\.settings-panel-icon\.telegram-icon/);
});

test("settings status rows can omit redundant icons without changing the default", () => {
  const shared = readFileSync(new URL("../src/pages/shared.jsx", import.meta.url), "utf8");
  assert.match(shared, /showIcon = true/);
  assert.match(settingsSource, /<StatusRow[^>]*showIcon=\{false\}[^>]*Sidecar/);
  assert.match(settingsSource, /<StatusRow[^>]*showIcon=\{false\}[^>]*Extension/);
});

test("Extension uses a puzzle icon distinct from the network proxy globe", () => {
  const icons = readFileSync(new URL("../src/components/icon.jsx", import.meta.url), "utf8");
  assert.match(settingsSource, /title="浏览器 Extension"[^>]*icon="extension"/);
  assert.match(settingsSource, /title="网络代理"[^>]*icon="browser"/);
  assert.match(icons, /extension:\s*"[^"]*a2\.5 2\.5 0 1 1[^"]*"/);
});

test("Telegram settings follow shared form styling and preserve their dedicated icon", () => {
  const telegram = readFileSync(new URL("../src/components/telegram-settings.jsx", import.meta.url), "utf8");
  const icons = readFileSync(new URL("../src/components/icon.jsx", import.meta.url), "utf8");
  assert.match(telegram, /icon="telegram"/);
  assert.match(icons, /telegram:/);
  assert.match(telegram, /className="settings-fields telegram-settings-grid"/);
  assert.match(telegram, /<label htmlFor="telegram-api-base">API 地址<\/label>/);
  assert.match(telegram, /type="password" autoComplete="new-password"/);
  assert.match(telegram, /description="配置发送目标、服务端点和凭据，管理归档发送服务。"/);
});

test("application icon assets cover every size Windows requests", () => {
  const generator = readFileSync(new URL("../scripts/make-icon.py", import.meta.url), "utf8");
  const sizeRow = generator.match(/^SIZES = \(([^)]*)\)/m)?.[1] ?? "";
  const sizes = sizeRow.split(",").map((value) => Number(value.trim())).filter(Boolean);
  // Windows picks the closest entry for the title bar, taskbar and Alt+Tab.
  for (const required of [16, 20, 24, 32, 40, 48, 64, 128, 256]) {
    assert.ok(sizes.includes(required), `SIZES is missing ${required}px`);
  }
  // Every primitive must be drawn at the supersampled resolution and the canvas
  // downscaled once, otherwise large sizes inherit low-resolution plate edges.
  const render = generator.match(/def render\(size: int\)[\s\S]*?\r?\n\r?\n/)?.[0] ?? "";
  assert.match(render, /rounded_square\(canvas\)/);
  assert.match(render, /draw_archive_box\(canvas\)/);
  assert.match(render, /draw_x_mark\(canvas\)/);
  assert.match(render, /resize\(\(size, size\), Image\.LANCZOS\)/);
  assert.doesNotMatch(render, /plate\.resize/);
  assert.doesNotMatch(render, /large = /);

  // The committed ICO must actually carry every size, not only the generator.
  const ico = readFileSync(new URL("../src-tauri/icons/icon.ico", import.meta.url));
  assert.equal(ico.readUInt16LE(0), 0, "ICO reserved field");
  assert.equal(ico.readUInt16LE(2), 1, "ICO type is icon");
  const count = ico.readUInt16LE(4);
  const entries = [];
  for (let index = 0; index < count; index += 1) {
    const offset = 6 + index * 16;
    entries.push([ico[offset] || 256, ico[offset + 1] || 256]);
  }
  for (const required of [16, 20, 24, 32, 40, 48, 64, 128, 256]) {
    assert.ok(entries.some(([w, h]) => w === required && h === required), `icon.ico is missing ${required}px`);
  }
  // Every entry stores its own full-resolution bitmap, so no size is a resize of
  // another one at runtime.
  for (let index = 0; index < count; index += 1) {
    const offset = 6 + index * 16;
    const bytes = ico.readUInt32LE(offset + 8);
    const start = ico.readUInt32LE(offset + 12);
    assert.equal(ico.slice(start, start + 8).toString("latin1"), "\x89PNG\r\n\x1a\n", `ICO entry ${index} is not PNG encoded`);
    assert.ok(bytes > 0);
  }
});

test("the dashboard navigation icon is distinct from the sidecar waveform", () => {
  const iconSource = readFileSync(new URL("../src/components/icon.jsx", import.meta.url), "utf8");
  const dashboard = iconSource.match(/dashboard:\s*\n?\s*"([^"]+)"/)?.[1] ?? "";
  const activity = iconSource.match(/activity:\s*\n?\s*"([^"]+)"/)?.[1] ?? "";
  assert.ok(dashboard, "a dashboard icon path is registered");
  assert.ok(activity, "the activity icon path is still registered");
  assert.notEqual(dashboard, activity);
  // The nav entry uses the dashboard glyph; the sidecar status keeps the waveform.
  assert.match(mainSource, /<NavItem icon="dashboard" label="工作台"/);
  assert.doesNotMatch(mainSource, /<NavItem icon="activity"/);
  assert.match(settingsSource, /sidecarReady \? "check" : "activity"/);
});

test("the icon generator extraction survives a CRLF checkout", () => {
  // A Windows checkout extracts the render() body with an LF-only blank-line
  // pattern, so the CRLF checkout failed 186/188 before 28543ed. The tolerant
  // pattern must cover both endings without widening into later functions.
  const generator = readFileSync(new URL("../scripts/make-icon.py", import.meta.url), "utf8");
  const lfOnly = /def render\(size: int\)[\s\S]*?\n\n/;
  const both = /def render\(size: int\)[\s\S]*?\r?\n\r?\n/;
  const lf = generator;
  const crlf = generator.replace(/\r?\n/g, "\r\n");

  assert.match(lf, both, "an LF checkout still matches the tolerant pattern");
  assert.match(crlf, both, "a CRLF checkout matches the tolerant pattern");
  // The LF-only pattern is exactly what failed on Windows; if it ever matches a
  // CRLF checkout again, the defect has returned unnoticed.
  assert.doesNotMatch(crlf, lfOnly, "the LF-only pattern must not match CRLF");

  // Tolerant, but still bounded by the first blank line.
  const captured = crlf.match(both)?.[0] ?? "";
  assert.doesNotMatch(captured, /def write_ico/);
  assert.doesNotMatch(captured, /def main/);
  assert.match(captured, /rounded_square\(canvas\)/);
  assert.match(captured, /resize\(\(size, size\), Image\.LANCZOS\)/);
});
