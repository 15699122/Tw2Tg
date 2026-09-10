import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { Badge } from "./components/ui/badge";
import { Button } from "./components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./components/ui/card";
import { Separator } from "./components/ui/separator";
import "./style.css";

const initialStatus = {
  app_name: "XArchive",
  app_version: "0.1.0",
  sidecar: "not_configured",
  database: "loading",
  platform: "unknown",
  archive_root: "loading",
  database_error: null,
  sidecar_error: null,
};

const initialAria2 = {
  found: false,
  version: null,
  path: null,
  source: null,
  error: null,
};

const statusLabels = {
  QUEUED: "排队中",
  VALIDATING: "校验中",
  METADATA_READY: "元数据就绪",
  DOWNLOADING: "下载中",
  DOWNLOADED: "已下载",
  COMPLETE: "已完成",
  FAILED: "失败",
  CANCELLED: "已取消",
  INTERRUPTED: "已中断",
};

const widgetErrorLabels = {
  status: "系统状态加载失败",
  jobs: "任务列表加载失败",
  aria2: "aria2 状态加载失败",
  folder: "归档文件夹打开失败",
  sidecar: "Sidecar 操作失败",
};

function userFacingError(widget, reason) {
  const detail = String(reason).trim();
  const label = widgetErrorLabels[widget] || "操作失败";
  return detail ? `${label}：${detail}` : label;
}

function Icon({ name, size = 18 }) {
  const paths = {
    archive: <><path d="M4 5h16v4H4z" /><path d="M6 9v10h12V9M10 13h4" /></>,
    activity: <><path d="M3 12h4l2-7 4 14 2-7h6" /></>,
    folder: <><path d="M3 6.5A1.5 1.5 0 0 1 4.5 5H9l2 2h8.5A1.5 1.5 0 0 1 21 8.5v8A1.5 1.5 0 0 1 19.5 18h-15A1.5 1.5 0 0 1 3 16.5z" /></>,
    refresh: <><path d="M20 11a8 8 0 0 0-14.8-4L3 10" /><path d="M3 5v5h5M4 13a8 8 0 0 0 14.8 4L21 14" /><path d="M21 19v-5h-5" /></>,
    download: <><path d="M12 3v12" /><path d="m7 10 5 5 5-5" /><path d="M5 21h14" /></>,
    play: <path d="m8 5 11 7-11 7z" fill="currentColor" stroke="none" />,
    stop: <rect x="6" y="6" width="12" height="12" rx="2" fill="currentColor" stroke="none" />,
    chevron: <path d="m9 6 6 6-6 6" />,
    check: <path d="m5 12 4 4L19 6" />,
  };
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">{paths[name]}</svg>;
}

function formatTime(value) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" }).format(date);
}

function App() {
  const [status, setStatus] = useState(initialStatus);
  const [jobs, setJobs] = useState([]);
  const [errors, setErrors] = useState({ status: "", jobs: "", aria2: "", folder: "", sidecar: "" });
  const [busy, setBusy] = useState(false);
  const [folderBusy, setFolderBusy] = useState(false);
  const [aria2, setAria2] = useState(initialAria2);
  const [aria2Releases, setAria2Releases] = useState([]);
  const [aria2Version, setAria2Version] = useState("1.37.0");
  const [aria2Busy, setAria2Busy] = useState(false);
  const [initialLoad, setInitialLoad] = useState(true);

  const setWidgetError = (widget, reason) => setErrors((current) => ({ ...current, [widget]: userFacingError(widget, reason) }));
  const clearWidgetError = (widget) => setErrors((current) => ({ ...current, [widget]: "" }));
  const refreshStatus = () => { clearWidgetError("status"); return invoke("get_app_status").then(setStatus).catch((reason) => setWidgetError("status", reason)); };
  const refreshJobs = () => { clearWidgetError("jobs"); return invoke("list_jobs", { limit: 20 }).then(setJobs).catch((reason) => setWidgetError("jobs", reason)); };
  const refreshAria2 = () => { clearWidgetError("aria2"); return invoke("detect_aria2").then(setAria2).catch((reason) => setWidgetError("aria2", reason)); };
  const loadAria2Releases = () => invoke("list_aria2_releases").then((items) => {
    setAria2Releases(items);
    if (items.length && !items.some((item) => item.version === aria2Version)) setAria2Version(items[0].version);
  }).catch((reason) => setWidgetError("aria2", reason));

  useEffect(() => {
    Promise.allSettled([refreshStatus(), refreshJobs(), refreshAria2(), loadAria2Releases()])
      .finally(() => setInitialLoad(false));
  }, []);

  const runSidecarCommand = (command) => {
    setBusy(true);
    clearWidgetError("sidecar");
    invoke(command)
      .then(() => Promise.all([refreshStatus(), refreshJobs()]))
      .catch((reason) => setWidgetError("sidecar", reason))
      .finally(() => setBusy(false));
  };

  const downloadAria2 = () => {
    setAria2Busy(true);
    clearWidgetError("aria2");
    invoke("download_aria2", { version: aria2Version })
      .then(() => refreshAria2())
      .catch((reason) => setWidgetError("aria2", reason))
      .finally(() => setAria2Busy(false));
  };

  const openArchiveFolder = () => {
    setFolderBusy(true);
    clearWidgetError("folder");
    invoke("open_archive_folder")
      .catch((reason) => setWidgetError("folder", reason))
      .finally(() => setFolderBusy(false));
  };

  const refreshAll = () => Promise.allSettled([refreshStatus(), refreshJobs(), ...(isWindows ? [refreshAria2()] : [])]);

  const completedJobs = jobs.filter((job) => job.state === "COMPLETE").length;
  const activeJobs = jobs.filter((job) => ["QUEUED", "VALIDATING", "DOWNLOADING"].includes(job.state)).length;
  const databaseReady = status.database === "ready";
  const sidecarReady = status.sidecar === "ready";
  const systemReady = sidecarReady && databaseReady;
  const isWindows = (status.platform || "").toLowerCase().includes("windows");

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-lockup">
          <div className="brand-mark"><Icon name="archive" size={18} /></div>
          <div><strong>XArchive</strong><span>LOCAL ARCHIVE</span></div>
        </div>
        <Separator />
        <nav className="nav-list" aria-label="主导航"><NavItem icon="activity" label="工作台" active /></nav>
        <div className="sidebar-footer">
          <p className="sidebar-caption">服务状态</p>
          <ConnectionStatus label="SQLite" ready={databaseReady} loading={initialLoad} />
          <ConnectionStatus label="Sidecar" ready={sidecarReady} loading={initialLoad} />
          <Separator />
          <span className="version-label">v{status.app_version} · {status.platform}</span>
        </div>
      </aside>
      <main className="main-panel">
        <header className="topbar">
          <div><p className="breadcrumb">XARCHIVE / WORKSPACE</p><h1>工作台</h1><p className="page-description">查看本地归档任务与运行环境。</p></div>
          <div className="topbar-actions"><Badge variant={systemReady ? "success" : "secondary"}><i className="status-pulse" />{systemReady ? "系统就绪" : "等待服务"}</Badge><Button variant="outline" size="sm" onClick={refreshAll}><Icon name="refresh" size={14} />刷新</Button></div>
        </header>
        <section className="summary-grid" aria-label="归档概览">
          <MetricCard label="最近任务" value={initialLoad ? "…" : jobs.length} detail="最近加载的 20 条" icon="archive" />
          <MetricCard label="进行中" value={initialLoad ? "…" : activeJobs} detail="等待或正在处理" icon="activity" />
          <MetricCard label="已完成" value={initialLoad ? "…" : completedJobs} detail="已保存到本地" icon="check" />
          <MetricCard label="数据库" value={initialLoad ? "…" : databaseReady ? "READY" : "ERROR"} detail="SQLite 状态" icon="folder" accent={databaseReady ? "green" : "red"} />
        </section>
        <div className="dashboard-grid">
          <Card className="jobs-panel">
            <CardHeader className="section-header"><div><CardTitle>最近任务</CardTitle><CardDescription>由 Rust 管理的本地归档任务</CardDescription></div><Button variant="ghost" size="sm" onClick={refreshJobs}><Icon name="refresh" size={14} />刷新</Button></CardHeader>
            <CardContent className="jobs-content">{errors.jobs ? <WidgetError message={errors.jobs} onRetry={refreshJobs} /> : initialLoad ? <LoadingJobs /> : jobs.length === 0 ? <EmptyJobs /> : <ul className="job-list">{jobs.map((job) => <JobRow key={job.job_id} job={job} />)}</ul>}</CardContent>
          </Card>
          <div className="side-column">
            <Card className="control-panel">
              <CardHeader><CardTitle>运行环境</CardTitle><CardDescription>本地 Sidecar 进程</CardDescription></CardHeader>
              <CardContent>{errors.sidecar && <WidgetError message={errors.sidecar} onRetry={() => runSidecarCommand(sidecarReady ? "stop_sidecar" : "start_sidecar")} />}<div className="runtime-status"><span className={`runtime-icon ${sidecarReady ? "ready" : ""}`}><Icon name="activity" size={18} /></span><div><strong>{sidecarReady ? "Sidecar 正在运行" : "Sidecar 未启动"}</strong><span>{sidecarReady ? "已完成 hello → ready 握手" : "启动后即可处理归档任务"}</span></div></div><div className="control-buttons"><Button className="grow" disabled={busy || sidecarReady} onClick={() => runSidecarCommand("start_sidecar")}><Icon name="play" size={14} />{busy ? "处理中…" : "启动 Sidecar"}</Button><Button className="grow" variant="outline" disabled={busy || !sidecarReady} onClick={() => runSidecarCommand("stop_sidecar")}><Icon name="stop" size={14} />停止</Button></div></CardContent>
            </Card>
            <Card className="location-panel"><CardHeader><CardTitle>归档位置</CardTitle><CardDescription>文件会先经过 staging 校验</CardDescription></CardHeader><CardContent>{errors.folder && <WidgetError message={errors.folder} onRetry={openArchiveFolder} />}<div className="path-display"><Icon name="folder" size={16} /><code title={status.archive_root}>{status.archive_root}</code></div><Button variant="outline" size="sm" className="location-button" disabled={folderBusy} onClick={openArchiveFolder}><Icon name="folder" size={14} />{folderBusy ? "正在打开…" : "打开文件夹"}</Button></CardContent></Card>
          </div>
        </div>
        {isWindows && <Aria2Panel installation={aria2} releases={aria2Releases} selectedVersion={aria2Version} busy={aria2Busy} error={errors.aria2} onVersionChange={setAria2Version} onRefresh={refreshAria2} onDownload={downloadAria2} />}
        {(status.database_error || status.sidecar_error || errors.status) && <div className="alert-box" role="alert" aria-live="assertive">{status.database_error || status.sidecar_error || errors.status}</div>}
        <footer className="footer-bar"><span>RUST OWNS THE STATE</span><span>协议版本 1.0</span><span>本地优先 · 隐私安全</span></footer>
      </main>
    </div>
  );
}

function NavItem({ icon, label, active }) { return <button type="button" className={`nav-item ${active ? "nav-item-active" : ""}`} disabled={!active} aria-label={label} aria-current={active ? "page" : undefined}><Icon name={icon} size={17} /><span>{label}</span>{active && <i className="nav-indicator" />}</button>; }

function ConnectionStatus({ label, ready, loading }) { return <div className="connection-line"><i className={`dot ${ready ? "dot-online" : loading ? "dot-muted" : "dot-error"}`} /><span>{label}</span><small>{ready ? "已连接" : loading ? "检测中…" : "异常"}</small></div>; }
function MetricCard({ label, value, detail, icon, accent = "default" }) { return <Card className={`metric-card metric-${accent}`}><div className="metric-top"><span>{label}</span><span className="metric-icon"><Icon name={icon} size={15} /></span></div><strong>{value}</strong><small>{detail}</small></Card>; }

function JobRow({ job }) { const variant = job.state === "COMPLETE" ? "success" : job.state === "FAILED" ? "destructive" : job.state === "DOWNLOADING" ? "warning" : "secondary"; return <li className="job-row"><span className={`job-type-mark ${job.state === "COMPLETE" ? "complete" : ""}`}><Icon name={job.state === "COMPLETE" ? "check" : "archive"} size={15} /></span><div className="job-main"><strong>Tweet {job.tweet_id}</strong><span>{job.job_id} · {job.tweet_type}</span></div><time className="job-time" dateTime={job.updated_at}>{formatTime(job.updated_at)}</time><Badge variant={variant}>{statusLabels[job.state] || job.state}</Badge></li>; }

function EmptyJobs() { return <div className="empty-jobs"><div className="empty-icon"><Icon name="archive" size={22} /></div><strong>还没有归档任务</strong><span>从浏览器提交一个 Tweet 后，任务会显示在这里。</span></div>; }

function LoadingJobs() { return <div className="empty-jobs" aria-label="正在加载任务"><div className="skeleton skeleton-icon" /><div className="skeleton skeleton-title" /><div className="skeleton skeleton-copy" /></div>; }

function WidgetError({ message, onRetry }) { return <div className="widget-error" role="alert"><span>{message}</span><Button variant="outline" size="sm" onClick={onRetry}>重试</Button></div>; }

function Aria2Panel({ installation, releases, selectedVersion, busy, error, onVersionChange, onRefresh, onDownload }) {
  const status = installation.found ? "已检测到" : "未检测到";
  const sourceLabels = {
    path: "PATH",
    program_directory: "程序目录",
    program_bin: "程序 bin 目录",
    program_data: "应用数据目录",
    program_data_bin: "应用数据 bin 目录",
    program_data_version: "应用数据版本目录",
    program_data_version_bin: "应用数据版本 bin 目录",
    program_data_build: "应用数据发行目录",
  };
  const source = sourceLabels[installation.source] || "可下载安装";
  return <Card className="aria2-panel">
    <CardHeader className="aria2-header">
      <div className="aria2-title-wrap"><div className="aria2-logo"><span>a</span><i>2</i></div><div><CardTitle>aria2</CardTitle><CardDescription>下载引擎版本管理</CardDescription></div></div>
      <Button variant="ghost" size="icon" aria-label="检测 aria2" onClick={onRefresh} disabled={busy}><Icon name="refresh" size={20} /></Button>
    </CardHeader>
    <CardContent>
      {error && <WidgetError message={error} onRetry={onRefresh} />}
      <div className="aria2-status-row"><Badge variant={installation.found ? "success" : "warning"}>{status}</Badge><span>{installation.version ? `v${installation.version}` : "当前程序未找到 aria2c"}</span><small>{source}</small></div>
      {installation.path && <code className="aria2-path" title={installation.path}>{installation.path}</code>}
      <div className="aria2-controls">
        <label htmlFor="aria2-version">选择版本</label>
        <select id="aria2-version" value={selectedVersion} onChange={(event) => onVersionChange(event.target.value)} disabled={busy || !releases.length}>
          {releases.map((release) => <option key={release.version} value={release.version}>v{release.version}</option>)}
        </select>
        <Button className="aria2-download-button" size="lg" onClick={onDownload} disabled={busy || !selectedVersion}>{busy ? "正在下载…" : installation.found ? "下载其他版本" : "下载并安装"}<Icon name="download" size={18} /></Button>
      </div>
      <p className="aria2-help">仅使用官方 aria2 Windows x64 发布包，下载后会校验 SHA-256，不覆盖 PATH 中已有程序。</p>
    </CardContent>
  </Card>;
}

createRoot(document.getElementById("root")).render(<React.StrictMode><App /></React.StrictMode>);