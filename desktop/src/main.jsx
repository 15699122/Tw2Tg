import { emitFrontendEvent, installFrontendBootstrap, markReactMounted, setStartupState } from "./bootstrap.js";
import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Separator } from "./components/ui/separator";
import Icon from "./components/icon.jsx";
import ConnectionStatus, { ExtensionConnectionStatus } from "./components/connection-status.jsx";
import { ComponentBootstrapStatus } from "./components/connection-status.jsx";
import DashboardPage from "./pages/dashboard-page.jsx";
import BatchesPage from "./pages/batches-page.jsx";
import SettingsPage from "./pages/settings-page.jsx";
import DownloadsPage from "./pages/download-settings-page.jsx";
import LogsPage from "./pages/logs-page.jsx";
import ErrorBoundary from "./components/error-boundary.jsx";
import "./style.css";

installFrontendBootstrap();
setStartupState("entry_module_evaluated");
if (import.meta.env.VITE_WDIO_E2E === "1") await import("@wdio/tauri-plugin");

const initialStatus = { app_name: "XArchive", app_version: "0.2.1", sidecar: "not_configured", database: "loading", platform: "unknown", archive_root: "loading", gallery_dl_path: null, logs_root: "loading", database_error: null, sidecar_error: null, download_setup_required: false, logging_level: "info", max_log_files: 5 };
const initialAria2 = { found: false, version: null, path: null, source: null, error: null };
const initialExtension = { files_ready: false, directory: "loading", browser_connection: "unknown", native_host: "unknown", message: "正在检测 Extension 文件。" };

function errorText(label, reason) { const detail = String(reason || "").trim(); return detail ? `${label}：${detail}` : label; }

function App() {
  const [page, setPage] = useState("dashboard");
  const [downloadsPage, setDownloadsPage] = useState(0);
  const [focusedJobId, setFocusedJobId] = useState("");
  const [expandedSettings, setExpandedSettings] = useState({});
  const toggleSettingsSection = (key) => setExpandedSettings((current) => ({ ...current, [key]: !current[key] }));
  const openSettingsSection = (key) => {
    setExpandedSettings((current) => ({ ...current, [key]: true }));
    setPage("settings");
    window.setTimeout(() => {
      const section = document.getElementById(`${key}-settings`);
      if (!section) return;
      section.scrollIntoView({ block: "nearest", behavior: "smooth" });
      (section.querySelector(".settings-panel-toggle") || section).focus();
    }, 0);
  };
  const [status, setStatus] = useState(initialStatus); const [jobs, setJobs] = useState([]); const [metrics, setMetrics] = useState({ total: 0, active: 0, completed: 0, failed: 0 }); const [aria2, setAria2] = useState(initialAria2); const [aria2CustomPath, setAria2CustomPath] = useState(""); const [aria2PathBusy, setAria2PathBusy] = useState(false); const [aria2PathMessage, setAria2PathMessage] = useState(""); const [sidecarPath, setSidecarPath] = useState(""); const [galleryDlPath, setGalleryDlPath] = useState(""); const [galleryDlMessage, setGalleryDlMessage] = useState(""); const [galleryDlBusy, setGalleryDlBusy] = useState(false); const [copied, setCopied] = useState(""); const [extension, setExtension] = useState(initialExtension);
  const [errors, setErrors] = useState({ status: "", jobs: "", aria2: "", sidecar: "", folder: "", extension: "" }); const [busy, setBusy] = useState(false); const [aria2Busy, setAria2Busy] = useState(false); const [extensionBusy, setExtensionBusy] = useState(false); const [folderBusy, setFolderBusy] = useState(false); const [setupBusy, setSetupBusy] = useState(false); const [settingsBusy, setSettingsBusy] = useState(false); const [settingsMessage, setSettingsMessage] = useState(""); const [loggingLevel, setLoggingLevel] = useState("info"); const [maxLogFiles, setMaxLogFiles] = useState(5); const [initialLoad, setInitialLoad] = useState(true);
  const [archiveBusy, setArchiveBusy] = useState(false); const [archiveMessage, setArchiveMessage] = useState("");
  const [useAria2, setUseAria2] = useState(false); const [useAria2Busy, setUseAria2Busy] = useState(false); const [useAria2Message, setUseAria2Message] = useState("");
  const [proxySettings, setProxySettings] = useState(null); const [proxyValue, setProxyValue] = useState(""); const [proxyBusy, setProxyBusy] = useState(false); const [proxyMessage, setProxyMessage] = useState(""); const [proxyRoute, setProxyRoute] = useState(null); const [proxySystem, setProxySystem] = useState(null); const [proxyDiagnoseUrl, setProxyDiagnoseUrl] = useState("https://api.telegram.org");
  const [bootstrap, setBootstrap] = useState(null);
  const [downloadPageData, setDownloadPageData] = useState({ jobs: [], total: 0 });
  const [downloadPageLoading, setDownloadPageLoading] = useState(false);
  const [downloadPageError, setDownloadPageError] = useState("");
  const [jobDetails, setJobDetails] = useState({});
  const [batches, setBatches] = useState([]); const [batchesLoading, setBatchesLoading] = useState(true); const [batchBusy, setBatchBusy] = useState(false); const [batchError, setBatchError] = useState("");
  const setError = (key, label, reason) => setErrors((current) => ({ ...current, [key]: errorText(label, reason) })); const clearError = (key) => setErrors((current) => ({ ...current, [key]: "" }));
  const refreshStatus = () => { clearError("status"); return invoke("get_app_status").then((next) => { setStatus(next); setLoggingLevel(next.logging_level || "info"); setMaxLogFiles(next.max_log_files || 5); setUseAria2(Boolean(next.use_aria2)); }).catch((reason) => setError("status", "系统状态加载失败", reason)); };
  const refreshJobs = () => { clearError("jobs"); return Promise.all([invoke("list_jobs", { limit: 20 }), invoke("get_job_metrics")]).then(([nextJobs, nextMetrics]) => { setJobs(nextJobs); setMetrics(nextMetrics); }).catch((reason) => setError("jobs", "任务列表加载失败", reason)); };
  const refreshDownloadPage = (offset = downloadsPage * 20) => { setDownloadPageLoading(true); setDownloadPageError(""); return invoke("list_jobs_page", { request: { offset, limit: 20 } }).then((result) => { setDownloadPageData(result); return result; }).catch((reason) => { setDownloadPageError(`下载任务加载失败：${String(reason)}`); return null; }).finally(() => setDownloadPageLoading(false)); };
  const openDownloadJob = (jobId, summaryIndex = -1) => {
    setFocusedJobId(jobId);
    setPage("downloads");
    setDownloadPageError("");
    void (async () => {
      try {
        const historyIndex = await invoke("get_job_history_index", { jobId });
        if (historyIndex == null) {
          setDownloadPageError("任务已不在当前记录中，请刷新后重试。");
          return;
        }
        const page = Math.floor(historyIndex / 20);
        const result = await invoke("list_jobs_page", { request: { offset: page * 20, limit: 20 } });
        setDownloadsPage(page);
        setDownloadPageData(result);
        if (!result.jobs.some((job) => job.job_id === jobId)) {
          setDownloadPageError("任务列表已变化，请刷新后重试。");
          return;
        }
        const detail = await invoke("get_job_download_metrics", { jobId });
        setJobDetails((current) => ({ ...current, [jobId]: detail }));
      } catch (reason) {
        setDownloadPageError(`任务详情加载失败：${String(reason)}`);
      }
    })();
  };
  const loadBatches = (showLoading) => { if (showLoading) { setBatchesLoading(true); clearError("batches"); } return invoke("list_account_batches", { limit: 20 }).then(setBatches).catch((reason) => setBatchError(`账号批次加载失败：${String(reason)}`)).finally(() => { if (showLoading) setBatchesLoading(false); }); };
  const refreshBatches = () => loadBatches(true);
  const refreshBatchesInBackground = () => loadBatches(false);
  const createBatch = (request) => { setBatchBusy(true); setBatchError(""); return invoke("create_account_batch", { request }).then(() => refreshBatches()).catch((reason) => setBatchError(`账号批次创建失败：${String(reason)}`)).finally(() => setBatchBusy(false)); };
  const controlBatch = (batchId, command) => { setBatchBusy(true); setBatchError(""); return invoke(command, { batchId }).then(() => refreshBatches()).catch((reason) => setBatchError(`账号批次操作失败：${String(reason)}`)).finally(() => setBatchBusy(false)); };
  const refreshAria2 = () => { clearError("aria2"); return invoke("detect_aria2").then(setAria2).catch((reason) => setError("aria2", "aria2 状态加载失败", reason)); };
  const refreshExtension = () => { setExtensionBusy(true); clearError("extension"); return invoke("get_extension_status").then(setExtension).catch((reason) => setError("extension", "Extension 状态加载失败", reason)).finally(() => setExtensionBusy(false)); };
  const manageNativeHost = (command) => { setExtensionBusy(true); clearError("extension"); return invoke(command).then(setExtension).catch((reason) => setError("extension", "Native Host 注册操作失败", reason)).finally(() => setExtensionBusy(false)); };
  const refreshBootstrap = () => invoke("get_component_bootstrap_status").then(setBootstrap).catch(() => setBootstrap(null));
  const loadSidecarPath = () => invoke("get_sidecar_path").then((path) => invoke("validate_gallery_dl_path", { path }).then((result) => { if (result.found) { setSidecarPath(result.path || path); setGalleryDlPath(result.path || path); } else { setSidecarPath(""); setGalleryDlPath(""); } })).catch(() => { setSidecarPath(""); setGalleryDlPath(""); });
  useEffect(() => {
    emitFrontendEvent("initial_ipc_started");
    Promise.allSettled([refreshStatus(), refreshJobs(), refreshDownloadPage(0), refreshAria2(), refreshExtension(), refreshBootstrap(), loadSidecarPath(), refreshBatches(), refreshProxy()])
      .finally(() => { setInitialLoad(false); emitFrontendEvent("initial_ipc_settled"); });
  }, []);
  useEffect(() => {
    // Browser/Native Host submissions do not originate in this React tree. A
    // bounded local poll discovers them without requiring a manual refresh and
    // stops on unmount; both commands are read-only SQLite projections.
    const interval = window.setInterval(() => {
      void refreshJobs();
      void refreshBatchesInBackground();
    }, 1500);
    return () => window.clearInterval(interval);
  }, []);
  const refreshAll = () => Promise.allSettled([refreshStatus(), refreshJobs(), refreshDownloadPage(), refreshAria2(), refreshExtension(), refreshBootstrap(), refreshBatches(), refreshProxy()]);
  const runSidecar = (command) => { setBusy(true); clearError("sidecar"); invoke(command).then(() => Promise.all([refreshStatus(), refreshJobs()])).catch((reason) => setError("sidecar", "Sidecar 操作失败", reason)).finally(() => setBusy(false)); };
  const downloadAria2 = () => { setAria2Busy(true); clearError("aria2"); invoke("download_aria2", { version: "" }).then(refreshAria2).catch((reason) => setError("aria2", "aria2 安装失败", reason)).finally(() => setAria2Busy(false)); };
  const copyPath = (key, value) => { if (!value) return; clearError(key); invoke("copy_text_to_clipboard", { text: String(value) }).then(() => { setCopied(key); window.setTimeout(() => setCopied((current) => (current === key ? "" : current)), 1600); }).catch((reason) => setError(key, "复制失败", reason)); };
  const checkAria2Path = () => { const value = aria2CustomPath.trim(); if (!value) { setAria2PathMessage("请输入 aria2c 可执行文件路径。"); return; } setAria2PathBusy(true); setAria2PathMessage(""); invoke("validate_aria2_path", { path: value }).then((result) => setAria2PathMessage(result.found ? "路径有效。" : result.error || "路径无效。")).catch((reason) => setAria2PathMessage(`校验失败：${String(reason)}`)).finally(() => setAria2PathBusy(false)); };
  const chooseExecutable = (setter, messageSetter, extensions) => { open({ multiple: false, directory: false, filters: [{ name: "Executable", extensions }] }).then((path) => { if (typeof path === "string") { setter(path); messageSetter(""); } }); };
  const saveAria2Path = () => { const value = aria2CustomPath.trim(); if (!value) { setAria2PathMessage("请选择 aria2c 可执行文件。"); return; } setAria2PathBusy(true); setAria2PathMessage(""); invoke("save_aria2_path", { path: value }).then((result) => { setAria2(result); setAria2CustomPath(result.path || value); setAria2PathMessage("aria2 路径已保存。"); }).catch((reason) => setAria2PathMessage(`保存失败：${String(reason)}`)).finally(() => setAria2PathBusy(false)); };
  const chooseGalleryDl = () => open({ multiple: false, directory: false, filters: [{ name: "Executable", extensions: ["exe", "py"] }] }).then((path) => { if (typeof path !== "string") return; setGalleryDlPath(path); setGalleryDlBusy(true); setGalleryDlMessage(""); invoke("save_gallery_dl_path", { path }).then((result) => { const savedPath = result.path || path; setGalleryDlPath(savedPath); setSidecarPath(savedPath); setGalleryDlMessage(`已检测到 gallery-dl v${result.version || "未知"}，路径已保存。`); }).catch((reason) => { setGalleryDlPath(""); setSidecarPath(""); setGalleryDlMessage(`校验失败，请重新选择：${String(reason)}`); }).finally(() => setGalleryDlBusy(false)); });
  const chooseAria2 = () => chooseExecutable(setAria2CustomPath, setAria2PathMessage, ["exe"]);
  // 切换传输后端：关闭时 gallery-dl 直接下载媒体，aria2 不再启动。
  const saveUseAria2 = (enabled) => { setUseAria2Busy(true); setUseAria2Message(""); invoke("set_use_aria2", { useAria2: enabled }).then((next) => { setStatus(next); setUseAria2(Boolean(next.use_aria2)); setUseAria2Message(enabled ? "已启用 aria2 传输；传输已重建，Extension 状态已刷新。" : "已切换为 gallery-dl 直接下载，aria2 不再启动；传输已重建，Extension 状态已刷新。"); void refreshExtension(); }).catch((reason) => { setUseAria2Message(`保存失败：${String(reason)}`); setUseAria2(Boolean(status.use_aria2)); }).finally(() => setUseAria2Busy(false)); };
  const openFolder = (command, key, label) => { setFolderBusy(true); clearError(key); invoke(command).catch((reason) => setError(key, label, reason)).finally(() => setFolderBusy(false)); };
  const chooseArchiveDirectory = async () => { setArchiveBusy(true); setArchiveMessage(""); try { const path = await open({ multiple: false, directory: true }); if (typeof path !== "string") return; const next = await invoke("set_archive_directory", { directory: path }); setStatus(next); setArchiveMessage(`归档目录已更新为 ${next.archive_root}。已有文件未移动。`); } catch (reason) { setArchiveMessage(`归档目录更新失败：${String(reason)}`); } finally { setArchiveBusy(false); } };
  const completeSetup = (choice) => { setSetupBusy(true); invoke("complete_download_setup", { choice }).then(() => Promise.all([refreshStatus(), refreshJobs()])).catch((reason) => setError("status", "下载目录设置失败", reason)).finally(() => setSetupBusy(false)); };
  const saveSettings = () => { const parsed = Number(maxLogFiles); if (!Number.isInteger(parsed) || parsed < 1 || parsed > 100) { setSettingsMessage("最大日志文件数必须是 1–100 之间的整数。"); return; } setSettingsBusy(true); setSettingsMessage(""); invoke("save_application_settings", { settings: { logging_level: loggingLevel, max_log_files: parsed } }).then((next) => { setStatus(next); setSettingsMessage("设置已保存。"); }).catch((reason) => setSettingsMessage(`设置保存失败：${String(reason)}`)).finally(() => setSettingsBusy(false)); };
  const databaseReady = status.database === "ready"; const sidecarReady = status.sidecar === "ready"; const isWindows = (status.platform || "").toLowerCase().includes("windows");
  const refreshProxy = () => Promise.all([invoke("get_network_settings").then(setProxySettings), invoke("get_system_proxy_status").then(setProxySystem)]).catch((reason) => setProxyMessage(`代理设置加载失败：${String(reason)}`));
  const saveProxy = () => { setProxyBusy(true); setProxyMessage(""); invoke("save_network_settings", { settings: { proxy_mode: proxySettings ? proxySettings.proxy_mode : "system", proxy: proxyValue } }).then((next) => { setProxySettings(next); setProxyValue(""); setProxyMessage("代理设置已保存，传输已重建；Extension 状态已刷新。"); void refreshExtension(); }).catch((reason) => setProxyMessage(`代理设置保存失败：${String(reason)}`)).finally(() => setProxyBusy(false)); };
  const inspectProxy = (url) => { setProxyBusy(true); setProxyMessage(""); invoke("inspect_proxy_route", { url }).then((next) => { setProxyRoute(next); if (next && next.system) setProxySystem(next.system); }).catch((reason) => setProxyMessage(`路由检测失败：${String(reason)}`)).finally(() => setProxyBusy(false)); };
  return (
    <div className="app-shell">
      <Sidebar
        page={page}
        setPage={setPage}
        openSettingsSection={openSettingsSection}
        status={status}
        databaseReady={databaseReady}
        sidecarReady={sidecarReady}
        extension={extension}
        extensionBusy={extensionBusy}
        initialLoad={initialLoad}
      />
      <main className="main-panel">
        <ErrorBoundary key={page} setPage={setPage}>
          {page === "dashboard" ? (
            <DashboardPage
              status={status}
              jobs={jobs}
              errors={errors}
              initialLoad={initialLoad}
              metrics={metrics}
              databaseReady={databaseReady}
              sidecarReady={sidecarReady}
              setupBusy={setupBusy}
              refreshAll={refreshAll}
              refreshJobs={refreshJobs}
              onShowAllJobs={() => { setDownloadsPage(0); setPage("downloads"); void refreshDownloadPage(0); }}
              openDownloadJob={openDownloadJob}
              extension={extension}
              bootstrap={bootstrap}
              aria2={aria2}
              useAria2={useAria2}
              completeSetup={completeSetup}
              setPage={setPage}
              openSettingsSection={openSettingsSection}
              runSidecar={runSidecar}
              busy={busy}
            />
          ) : page === "batches" ? (
            <BatchesPage batches={batches} loading={batchesLoading} busy={batchBusy} error={batchError} onRefresh={refreshBatches} onCreate={createBatch} onControl={controlBatch} />
          ) : page === "logs" ? (
            // 日志页的显示过滤器跟随后端 effective level，而不是写死 `info`。
            // `status.logging_level` 来自 `effective_level()`，所以预发布渠道的
            // debug 不会被页面自己藏起来；设置页保存后该值也会同步更新。
            <LogsPage loggingLevel={status.logging_level} />
          ) : page === "downloads" ? (
            <DownloadsPage
              jobs={downloadPageData.jobs}
              totalJobs={downloadPageData.total}
              jobsLoading={downloadPageLoading}
              jobsError={downloadPageError}
              jobsPage={downloadsPage}
              setJobsPage={(next) => { setDownloadsPage(next); void refreshDownloadPage(next * 20); }}
              refreshDownloadPage={() => refreshDownloadPage()}
              jobDetails={jobDetails}
              focusedJobId={focusedJobId}
              clearFocusedJob={() => setFocusedJobId("")}
              archiveRoot={status.archive_root}
              archiveBusy={archiveBusy}
              chooseArchiveDirectory={chooseArchiveDirectory}
              archiveError={errors.folder}
              isWindows={isWindows}
              useAria2={useAria2}
              useAria2Busy={useAria2Busy}
              useAria2Message={useAria2Message}
              saveUseAria2={saveUseAria2}
              aria2={aria2}
              aria2Busy={aria2Busy}
              aria2CustomPath={aria2CustomPath}
              aria2PathBusy={aria2PathBusy}
              aria2PathMessage={aria2PathMessage}
              refreshAria2={refreshAria2}
              downloadAria2={downloadAria2}
              checkAria2Path={checkAria2Path}
              saveAria2Path={saveAria2Path}
              chooseAria2={chooseAria2}
              errors={errors}
              copyPath={copyPath}
              copied={copied}
              expandedSections={expandedSettings}
              toggleSettingsSection={toggleSettingsSection}
            />
          ) : (
            <SettingsPage
              status={status}
              errors={errors}
              sidecarReady={sidecarReady}
              busy={busy}
              runSidecar={runSidecar}
              extension={extension}
              extensionBusy={extensionBusy}
              registerNativeHost={() => manageNativeHost("register_native_host")}
              unregisterNativeHost={() => manageNativeHost("unregister_native_host")}
              bootstrap={bootstrap}
              refreshExtension={refreshExtension}
              isWindows={isWindows}
              sidecarPath={sidecarPath}
              galleryDlPath={status.gallery_dl_path || galleryDlPath}
              copyPath={copyPath}
              copied={copied}
              loggingLevel={loggingLevel}
              setLoggingLevel={setLoggingLevel}
              maxLogFiles={maxLogFiles}
              setMaxLogFiles={setMaxLogFiles}
              settingsBusy={settingsBusy}
              settingsMessage={settingsMessage}
              saveSettings={saveSettings}
              folderBusy={folderBusy}
              openFolder={openFolder}
              proxySettings={proxySettings}
              proxyValue={proxyValue}
              setProxyValue={setProxyValue}
              setProxyMode={(mode) => setProxySettings((current) => ({ ...(current || { proxy_mode: mode }), proxy_mode: mode }))}
              proxyBusy={proxyBusy}
              proxyMessage={proxyMessage}
              saveProxy={saveProxy}
              inspectProxy={inspectProxy}
              proxyRoute={proxyRoute}
              proxySystem={proxySystem}
              proxyDiagnoseUrl={proxyDiagnoseUrl}
              setProxyDiagnoseUrl={setProxyDiagnoseUrl}
              expandedSections={expandedSettings}
              toggleSettingsSection={toggleSettingsSection}
            />
          )}
        </ErrorBoundary>
      </main>
    </div>
  );
}

function Sidebar({ page, setPage, openSettingsSection, status, databaseReady, sidecarReady, extension, extensionBusy, initialLoad }) {
  return (
    <aside className="sidebar">
      <div className="brand-lockup">
        <div className="brand-mark"><Icon name="archive" size={18} /></div>
        <div><strong>XArchive</strong><span>LOCAL ARCHIVE</span></div>
      </div>
      <Separator />
      <nav className="nav-list" aria-label="主导航">
        <NavItem icon="dashboard" label="工作台" active={page === "dashboard"} onClick={() => setPage("dashboard")} />
        <NavItem icon="archive" label="账号归档" active={page === "batches"} onClick={() => setPage("batches")} />
        <NavItem icon="file" label="运行日志" active={page === "logs"} onClick={() => setPage("logs")} />
        <NavItem icon="download" label="下载任务与设置" active={page === "downloads"} onClick={() => { setPage("downloads"); void refreshDownloadPage(); }} />
      </nav>
      <div className="sidebar-spacer" />
      <Separator className="sidebar-settings-separator" />
      <nav className="nav-list sidebar-settings-nav" aria-label="设置导航">
        <NavItem icon="settings" label="设置" active={page === "settings"} onClick={() => setPage("settings")} />
      </nav>
      <div className="sidebar-footer">
        <p className="sidebar-caption">服务状态</p>
        <ConnectionStatus label="SQLite" ready={databaseReady} loading={initialLoad} onClick={() => { setPage("downloads"); void refreshDownloadPage(); }} />
        <ConnectionStatus label="Sidecar" ready={sidecarReady} loading={initialLoad} onClick={() => openSettingsSection("sidecar")} />
        <ExtensionConnectionStatus extension={extension} initialLoad={initialLoad} checking={extensionBusy} onClick={() => openSettingsSection("extension")} />
        <Separator className="sidebar-footer-separator" />
        <span className="version-label">v{status.app_version} · {status.platform}</span>
      </div>
    </aside>
  );
}
function NavItem({ icon, label, active, onClick }) { return <button type="button" className={`nav-item ${active ? "nav-item-active" : ""}`} onClick={onClick} aria-current={active ? "page" : undefined}><Icon name={icon} size={17} /><span>{label}</span>{active && <i className="nav-indicator" />}</button>; }


setStartupState("react_mount_started");
createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <ErrorBoundary setPage={() => {}}>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
markReactMounted();
