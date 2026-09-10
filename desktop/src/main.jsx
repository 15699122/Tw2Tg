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

function App() {
  const [status, setStatus] = useState(initialStatus);
  const [jobs, setJobs] = useState([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [folderBusy, setFolderBusy] = useState(false);
  const [aria2, setAria2] = useState(initialAria2);
  const [aria2Releases, setAria2Releases] = useState([]);
  const [aria2Version, setAria2Version] = useState("1.37.0");
  const [aria2Busy, setAria2Busy] = useState(false);
  const [initialLoad, setInitialLoad] = useState(true);

  const refreshStatus = () => invoke("get_app_status").then(setStatus).catch((reason) => setError(String(reason)));
  const refreshJobs = () => invoke("list_jobs", { limit: 20 }).then(setJobs).catch((reason) => setError(String(reason)));
  const refreshAria2 = () => invoke("detect_aria2").then(setAria2).catch((reason) => setError(String(reason)));
  const loadAria2Releases = () => invoke("list_aria2_releases").then((items) => {
    setAria2Releases(items);
    if (items.length && !items.some((item) => item.version === aria2Version)) setAria2Version(items[0].version);
  }).catch((reason) => setError(String(reason)));

  useEffect(() => {
    Promise.allSettled([refreshStatus(), refreshJobs(), refreshAria2(), loadAria2Releases()])
      .finally(() => setInitialLoad(false));
  }, []);

  const runSidecarCommand = (command) => {
    setBusy(true);
    setError("");
    invoke(command)
      .then(() => Promise.all([refreshStatus(), refreshJobs()]))
      .catch((reason) => setError(String(reason)))
      .finally(() => setBusy(false));
  };

  const downloadAria2 = () => {
    setAria2Busy(true);
    setError("");
    invoke("download_aria2", { version: aria2Version })
      .then(() => refreshAria2())
      .catch((reason) => setError(String(reason)))
      .finally(() => setAria2Busy(false));
  };

  const openArchiveFolder = () => {
    setFolderBusy(true);
    setError("");
    invoke("open_archive_folder")
      .catch((reason) => setError(String(reason)))
      .finally(() => setFolderBusy(false));
  };

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
          <div className="brand-mark"><Icon name="archive" size={20} /></div>
          <div><strong>XArchive</strong><span>LOCAL ARCHIVE</span></div>
        </div>
        <Separator />
        <nav className="nav-list" aria-label="主导航">
          <NavItem icon="activity" label="工作台" active />
          <NavItem icon="archive" label="归档库" />
          <NavItem icon="folder" label="文件位置" />
        </nav>
        <div className="sidebar-footer">
          <div className="connection-line"><i className={databaseReady ? "dot dot-online" : initialLoad ? "dot dot-muted" : "dot dot-error"} /> SQLite {databaseReady ? "已连接" : initialLoad ? "连接中…" : "异常"}</div>
          <div className="connection-line"><i className={sidecarReady ? "dot dot-online" : "dot dot-muted"} /> Sidecar {sidecarReady ? "运行中" : initialLoad ? "检测中…" : "未启动"}</div>
          <Separator />
          <span className="version-label">v{status.app_version} · {status.platform}</span>
        </div>
      </aside>

      <main className="main-panel">
        <header className="topbar">
          <div><p className="breadcrumb">WORKSPACE / OVERVIEW</p><h1>归档工作台</h1></div>
          <div className="topbar-actions"><Badge variant={systemReady ? "success" : "secondary"}><i className="status-pulse" />{systemReady ? "系统就绪" : "等待中"}</Badge><Button variant="ghost" size="icon" aria-label="刷新任务" onClick={refreshJobs}><Icon name="refresh" /></Button></div>
        </header>

        <section className="welcome-row">
          <div><p className="kicker">TODAY'S ARCHIVE DESK</p><h2>保持本地，掌握每一份记录。</h2><p className="welcome-copy">从 X 页面接收任务，使用 Rust 校验并把原始媒体安全保存到本地归档。</p></div>
          <div className="hero-accent"><span>01</span><small>LOCAL<br />FIRST</small></div>
        </section>

        <section className="metric-grid" aria-label="归档概览">
          <MetricCard label="最近任务" value={initialLoad ? "…" : jobs.length} detail="最近 20 条" icon="archive" />
          <MetricCard label="进行中" value={initialLoad ? "…" : activeJobs} detail="等待处理" icon="activity" accent="amber" />
          <MetricCard label="已完成" value={initialLoad ? "…" : completedJobs} detail="本地归档" icon="check" accent="green" />
          <MetricCard label="数据库" value={initialLoad ? "…" : databaseReady ? "READY" : "ERROR"} detail="SQLite 状态" icon="folder" accent={initialLoad ? "default" : databaseReady ? "green" : "red"} />
        </section>

        {isWindows && <Aria2Panel
          installation={aria2}
          releases={aria2Releases}
          selectedVersion={aria2Version}
          busy={aria2Busy}
          onVersionChange={setAria2Version}
          onRefresh={refreshAria2}
          onDownload={downloadAria2}
        />}

        <div className="content-grid">
          <Card className="jobs-panel">
            <CardHeader className="jobs-header"><div><CardTitle>最近任务</CardTitle><CardDescription>由 Rust 管理的本地归档任务</CardDescription></div><Button variant="ghost" size="sm" onClick={refreshJobs}><Icon name="refresh" size={14} />刷新</Button></CardHeader>
            <CardContent>
              {initialLoad ? <LoadingJobs /> : jobs.length === 0 ? <EmptyJobs /> : <div className="job-list">{jobs.map((job) => <JobRow key={job.job_id} job={job} />)}</div>}
            </CardContent>
          </Card>

          <div className="side-column">
            <Card className="control-panel"><CardHeader><CardTitle>运行控制</CardTitle><CardDescription>Python Sidecar 进程</CardDescription></CardHeader><CardContent><div className="runtime-status"><div className={sidecarReady ? "runtime-orb ready" : "runtime-orb"}><Icon name="activity" size={22} /></div><div><strong>{sidecarReady ? "Sidecar 正在运行" : "Sidecar 未启动"}</strong><span>{sidecarReady ? "已完成 hello → ready 握手" : "配置后可启动下载进程"}</span></div></div><div className="control-buttons"><Button className="grow" disabled={busy || sidecarReady} onClick={() => runSidecarCommand("start_sidecar")}><Icon name="play" size={15} />{busy ? "处理中…" : "启动 Sidecar"}</Button><Button className="grow" variant="secondary" disabled={busy || !sidecarReady} onClick={() => runSidecarCommand("stop_sidecar")}><Icon name="stop" size={15} />停止</Button></div></CardContent></Card>
            <Card className="location-panel"><CardHeader><CardTitle>归档位置</CardTitle><CardDescription>所有文件都会先经过 staging 校验</CardDescription></CardHeader><CardContent><div className="path-display"><Icon name="folder" size={17} /><code>{status.archive_root}</code></div><Button variant="outline" size="sm" className="location-button" disabled={folderBusy} onClick={openArchiveFolder}><Icon name="folder" size={14} />{folderBusy ? "正在打开…" : "打开文件夹"}</Button></CardContent></Card>
          </div>
        </div>

        {(status.database_error || status.sidecar_error || error) && <div className="alert-box" role="alert" aria-live="assertive">{status.database_error || status.sidecar_error || error}</div>}
        <footer className="footer-bar"><span>RUST OWNS THE STATE</span><span>协议版本 1.0</span><span>本地优先 · 隐私安全</span></footer>
      </main>
    </div>
  );
}

function NavItem({ icon, label, active }) { return <button type="button" className={`nav-item ${active ? "nav-item-active" : ""}`} disabled={!active} aria-label={label} aria-current={active ? "page" : undefined}><Icon name={icon} size={17} /><span>{label}</span>{active && <i className="nav-indicator" />}</button>; }

function MetricCard({ label, value, detail, icon, accent = "default" }) { return <Card className={`metric-card metric-${accent}`}><div className="metric-top"><span>{label}</span><div className="metric-icon"><Icon name={icon} size={16} /></div></div><strong>{value}</strong><small>{detail}</small></Card>; }

function JobRow({ job }) { const variant = job.state === "COMPLETE" ? "success" : job.state === "FAILED" ? "destructive" : job.state === "DOWNLOADING" ? "warning" : "secondary"; return <div className="job-row"><div className="job-type-mark"><Icon name={job.state === "COMPLETE" ? "check" : "archive"} size={16} /></div><div className="job-main"><strong>Tweet {job.tweet_id}</strong><span>{job.job_id} · {job.tweet_type}</span></div><div className="job-time">{job.updated_at}</div><Badge variant={variant}>{statusLabels[job.state] || job.state}</Badge></div>; }

function EmptyJobs() { return <div className="empty-jobs"><div className="empty-icon"><Icon name="archive" size={22} /></div><strong>还没有归档任务</strong><span>从浏览器提交一个 Tweet 后，任务会显示在这里。</span></div>; }

function LoadingJobs() { return <div className="empty-jobs"><div className="empty-icon loading"><Icon name="refresh" size={22} /></div><strong>正在加载任务…</strong><span>请稍候</span></div>; }

function Aria2Panel({ installation, releases, selectedVersion, busy, onVersionChange, onRefresh, onDownload }) {
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