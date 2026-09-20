import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Separator } from "./components/ui/separator";
import Icon from "./components/icon.jsx";
import ConnectionStatus, { ExtensionConnectionStatus } from "./components/connection-status.jsx";
import { ComponentBootstrapStatus } from "./components/connection-status.jsx";
import DashboardPage from "./pages/dashboard-page.jsx";
import SettingsPage from "./pages/settings-page.jsx";
import LogsPage from "./pages/logs-page.jsx";
import ErrorBoundary from "./components/error-boundary.jsx";
import "./style.css";

if (import.meta.env.VITE_WDIO_E2E === "1") await import("@wdio/tauri-plugin");

const initialStatus = { app_name: "XArchive", app_version: "0.1.0", sidecar: "not_configured", database: "loading", platform: "unknown", archive_root: "loading", logs_root: "loading", database_error: null, sidecar_error: null, download_setup_required: false, logging_level: "info", max_log_files: 5 };
const initialAria2 = { found: false, version: null, path: null, source: null, error: null };
const initialExtension = { files_ready: false, directory: "loading", browser_connection: "unknown", native_host: "unknown", message: "正在检测 Extension 文件。" };

function errorText(label, reason) { const detail = String(reason || "").trim(); return detail ? `${label}：${detail}` : label; }

function App() {
  const [page, setPage] = useState("dashboard");
  const [status, setStatus] = useState(initialStatus); const [jobs, setJobs] = useState([]); const [metrics, setMetrics] = useState({ total: 0, active: 0, completed: 0, failed: 0 }); const [aria2, setAria2] = useState(initialAria2); const [aria2CustomPath, setAria2CustomPath] = useState(""); const [aria2PathBusy, setAria2PathBusy] = useState(false); const [aria2PathMessage, setAria2PathMessage] = useState(""); const [sidecarPath, setSidecarPath] = useState(""); const [galleryDlPath, setGalleryDlPath] = useState(""); const [galleryDlMessage, setGalleryDlMessage] = useState(""); const [galleryDlBusy, setGalleryDlBusy] = useState(false); const [copied, setCopied] = useState(""); const [extension, setExtension] = useState(initialExtension);
  const [errors, setErrors] = useState({ status: "", jobs: "", aria2: "", sidecar: "", folder: "", extension: "" }); const [busy, setBusy] = useState(false); const [aria2Busy, setAria2Busy] = useState(false); const [folderBusy, setFolderBusy] = useState(false); const [setupBusy, setSetupBusy] = useState(false); const [settingsBusy, setSettingsBusy] = useState(false); const [settingsMessage, setSettingsMessage] = useState(""); const [loggingLevel, setLoggingLevel] = useState("info"); const [maxLogFiles, setMaxLogFiles] = useState(5); const [initialLoad, setInitialLoad] = useState(true);
  const [bootstrap, setBootstrap] = useState(null);
  const setError = (key, label, reason) => setErrors((current) => ({ ...current, [key]: errorText(label, reason) })); const clearError = (key) => setErrors((current) => ({ ...current, [key]: "" }));
  const refreshStatus = () => { clearError("status"); return invoke("get_app_status").then((next) => { setStatus(next); setLoggingLevel(next.logging_level || "info"); setMaxLogFiles(next.max_log_files || 5); }).catch((reason) => setError("status", "系统状态加载失败", reason)); };
  const refreshJobs = () => { clearError("jobs"); return Promise.all([invoke("list_jobs", { limit: 20 }), invoke("get_job_metrics")]).then(([nextJobs, nextMetrics]) => { setJobs(nextJobs); setMetrics(nextMetrics); }).catch((reason) => setError("jobs", "任务列表加载失败", reason)); };
  const refreshAria2 = () => { clearError("aria2"); return invoke("detect_aria2").then(setAria2).catch((reason) => setError("aria2", "aria2 状态加载失败", reason)); };
  const refreshExtension = () => { clearError("extension"); return invoke("get_extension_status").then(setExtension).catch((reason) => setError("extension", "Extension 状态加载失败", reason)); };
  const refreshBootstrap = () => invoke("get_component_bootstrap_status").then(setBootstrap).catch(() => setBootstrap(null));
  const loadSidecarPath = () => invoke("get_sidecar_path").then((path) => invoke("validate_gallery_dl_path", { path }).then((result) => { if (result.found) { setSidecarPath(result.path || path); setGalleryDlPath(result.path || path); } else { setSidecarPath(""); setGalleryDlPath(""); } })).catch(() => { setSidecarPath(""); setGalleryDlPath(""); });
  useEffect(() => { Promise.allSettled([refreshStatus(), refreshJobs(), refreshAria2(), refreshExtension(), refreshBootstrap(), loadSidecarPath()]).finally(() => setInitialLoad(false)); }, []);
  const refreshAll = () => Promise.allSettled([refreshStatus(), refreshJobs(), refreshAria2(), refreshExtension(), refreshBootstrap()]);
  const runSidecar = (command) => { setBusy(true); clearError("sidecar"); invoke(command).then(() => Promise.all([refreshStatus(), refreshJobs()])).catch((reason) => setError("sidecar", "Sidecar 操作失败", reason)).finally(() => setBusy(false)); };
  const downloadAria2 = () => { setAria2Busy(true); clearError("aria2"); invoke("download_aria2", { version: "" }).then(refreshAria2).catch((reason) => setError("aria2", "aria2 安装失败", reason)).finally(() => setAria2Busy(false)); };
  const copyPath = (key, value) => { if (!value) return; clearError(key); invoke("copy_text_to_clipboard", { text: String(value) }).then(() => { setCopied(key); window.setTimeout(() => setCopied((current) => (current === key ? "" : current)), 1600); }).catch((reason) => setError(key, "复制失败", reason)); };
  const checkAria2Path = () => { const value = aria2CustomPath.trim(); if (!value) { setAria2PathMessage("请输入 aria2c 可执行文件路径。"); return; } setAria2PathBusy(true); setAria2PathMessage(""); invoke("validate_aria2_path", { path: value }).then((result) => setAria2PathMessage(result.found ? "路径有效。" : result.error || "路径无效。")).catch((reason) => setAria2PathMessage(`校验失败：${String(reason)}`)).finally(() => setAria2PathBusy(false)); };
  const chooseExecutable = (setter, messageSetter, extensions) => { open({ multiple: false, directory: false, filters: [{ name: "Executable", extensions }] }).then((path) => { if (typeof path === "string") { setter(path); messageSetter(""); } }); };
  const saveAria2Path = () => { const value = aria2CustomPath.trim(); if (!value) { setAria2PathMessage("请选择 aria2c 可执行文件。"); return; } setAria2PathBusy(true); setAria2PathMessage(""); invoke("save_aria2_path", { path: value }).then((result) => { setAria2(result); setAria2CustomPath(result.path || value); setAria2PathMessage("aria2 路径已保存。"); }).catch((reason) => setAria2PathMessage(`保存失败：${String(reason)}`)).finally(() => setAria2PathBusy(false)); };
  const chooseGalleryDl = () => open({ multiple: false, directory: false, filters: [{ name: "Executable", extensions: ["exe", "py"] }] }).then((path) => { if (typeof path !== "string") return; setGalleryDlPath(path); setGalleryDlBusy(true); setGalleryDlMessage(""); invoke("save_gallery_dl_path", { path }).then((result) => { const savedPath = result.path || path; setGalleryDlPath(savedPath); setSidecarPath(savedPath); setGalleryDlMessage(`已检测到 gallery-dl v${result.version || "未知"}，路径已保存。`); }).catch((reason) => { setGalleryDlPath(""); setSidecarPath(""); setGalleryDlMessage(`校验失败，请重新选择：${String(reason)}`); }).finally(() => setGalleryDlBusy(false)); });
  const chooseAria2 = () => chooseExecutable(setAria2CustomPath, setAria2PathMessage, ["exe"]);
  const openFolder = (command, key, label) => { setFolderBusy(true); clearError(key); invoke(command).catch((reason) => setError(key, label, reason)).finally(() => setFolderBusy(false)); };
  const completeSetup = (choice) => { setSetupBusy(true); invoke("complete_download_setup", { choice }).then(() => Promise.all([refreshStatus(), refreshJobs()])).catch((reason) => setError("status", "下载目录设置失败", reason)).finally(() => setSetupBusy(false)); };
  const saveSettings = () => { const parsed = Number(maxLogFiles); if (!Number.isInteger(parsed) || parsed < 1 || parsed > 100) { setSettingsMessage("最大日志文件数必须是 1–100 之间的整数。"); return; } setSettingsBusy(true); setSettingsMessage(""); invoke("save_application_settings", { settings: { logging_level: loggingLevel, max_log_files: parsed } }).then((next) => { setStatus(next); setSettingsMessage("设置已保存。"); }).catch((reason) => setSettingsMessage(`设置保存失败：${String(reason)}`)).finally(() => setSettingsBusy(false)); };
  const databaseReady = status.database === "ready"; const sidecarReady = status.sidecar === "ready"; const isWindows = (status.platform || "").toLowerCase().includes("windows");
  return (
    <div className="app-shell">
      <Sidebar
        page={page}
        setPage={setPage}
        status={status}
        databaseReady={databaseReady}
        sidecarReady={sidecarReady}
        extension={extension}
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
              completeSetup={completeSetup}
              setPage={setPage}
              runSidecar={runSidecar}
              busy={busy}
            />
          ) : page === "logs" ? (
            <LogsPage />
          ) : (
            <SettingsPage
              aria2Busy={aria2Busy}
              status={status}
              errors={errors}
              sidecarReady={sidecarReady}
              busy={busy}
              runSidecar={runSidecar}
              extension={extension}
              bootstrap={bootstrap}
              refreshExtension={refreshExtension}
              isWindows={isWindows}
              aria2={aria2}
              aria2CustomPath={aria2CustomPath}
              aria2PathBusy={aria2PathBusy}
              aria2PathMessage={aria2PathMessage}
              refreshAria2={refreshAria2}
              downloadAria2={downloadAria2}
              checkAria2Path={checkAria2Path}
              saveAria2Path={saveAria2Path}
              sidecarPath={sidecarPath}
              galleryDlPath={galleryDlPath}
              galleryDlMessage={galleryDlMessage}
              galleryDlBusy={galleryDlBusy}
              chooseGalleryDl={chooseGalleryDl}
              chooseAria2={chooseAria2}
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
            />
          )}
        </ErrorBoundary>
      </main>
    </div>
  );
}

function Sidebar({ page, setPage, status, databaseReady, sidecarReady, extension, initialLoad }) {
  return (
    <aside className="sidebar">
      <div className="brand-lockup">
        <div className="brand-mark"><Icon name="archive" size={18} /></div>
        <div><strong>XArchive</strong><span>LOCAL ARCHIVE</span></div>
      </div>
      <Separator />
      <nav className="nav-list" aria-label="主导航">
        <NavItem icon="activity" label="工作台" active={page === "dashboard"} onClick={() => setPage("dashboard")} />
        <NavItem icon="file" label="运行日志" active={page === "logs"} onClick={() => setPage("logs")} />
      </nav>
      <div className="sidebar-spacer" />
      <Separator />
      <nav className="nav-list" aria-label="设置导航">
        <NavItem icon="settings" label="设置" active={page === "settings"} onClick={() => setPage("settings")} />
      </nav>
      <div className="sidebar-footer">
        <p className="sidebar-caption">服务状态</p>
        <ConnectionStatus label="SQLite" ready={databaseReady} loading={initialLoad} onClick={() => { setPage("settings"); window.setTimeout(() => document.getElementById("storage-settings")?.focus(), 0); }} />
        <ConnectionStatus label="Sidecar" ready={sidecarReady} loading={initialLoad} onClick={() => { setPage("settings"); window.setTimeout(() => document.getElementById("sidecar-settings")?.focus(), 0); }} />
        <ExtensionConnectionStatus extension={extension} initialLoad={initialLoad} onClick={() => { setPage("settings"); window.setTimeout(() => document.getElementById("extension-settings")?.focus(), 0); }} />
        <Separator />
        <span className="version-label">v{status.app_version} · {status.platform}</span>
      </div>
    </aside>
  );
}
function NavItem({ icon, label, active, onClick }) { return <button type="button" className={`nav-item ${active ? "nav-item-active" : ""}`} onClick={onClick} aria-current={active ? "page" : undefined}><Icon name={icon} size={17} /><span>{label}</span>{active && <i className="nav-indicator" />}</button>; }


createRoot(document.getElementById("root")).render(<React.StrictMode><App /></React.StrictMode>);