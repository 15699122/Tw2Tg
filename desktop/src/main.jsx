import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
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

function App() {
  const [status, setStatus] = useState(initialStatus);
  const [jobs, setJobs] = useState([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const refreshStatus = () => {
    invoke("get_app_status")
      .then((value) => setStatus(value))
      .catch((reason) => setError(String(reason)));
  };

  const refreshJobs = () => {
    invoke("list_jobs", { limit: 20 })
      .then((value) => setJobs(value))
      .catch((reason) => setError(String(reason)));
  };

  useEffect(() => {
    let active = true;
    invoke("get_app_status")
      .then((value) => {
        if (active) setStatus(value);
      })
      .catch((reason) => {
        if (active) setError(String(reason));
      });
    invoke("list_jobs", { limit: 20 })
      .then((value) => {
        if (active) setJobs(value);
      })
      .catch((reason) => {
        if (active) setError(String(reason));
      });
    return () => {
      active = false;
    };
  }, []);

  const runSidecarCommand = (command) => {
    setBusy(true);
    setError("");
    invoke(command)
      .then(() => {
        refreshStatus();
        refreshJobs();
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setBusy(false));
  };

  return (
    <main className="shell">
      <header className="header">
        <div>
          <p className="eyebrow">LOCAL-FIRST ARCHIVE</p>
          <h1>XArchive</h1>
          <p className="subtitle">X/Twitter 本地归档控制台</p>
        </div>
        <span className="badge">DESKTOP RUNTIME</span>
      </header>

      <section className="hero-card">
        <div>
          <p className="eyebrow">SYSTEM STATUS</p>
          <h2>准备接收归档任务</h2>
          <p>后续将从这里管理 Job、Sidecar、SQLite、Telegram 和浏览器连接。</p>
        </div>
        <div className="status-mark">✓</div>
      </section>

      <section className="grid" aria-label="Application status">
        <StatusCard label="应用版本" value={status.app_version} />
        <StatusCard label="平台" value={status.platform} />
        <StatusCard label="Python Sidecar" value={status.sidecar} />
        <StatusCard label="SQLite" value={status.database} />
      </section>

      <section className="path-card">
        <span>归档根目录</span>
        <code>{status.archive_root}</code>
      </section>

      <section className="controls" aria-label="Sidecar controls">
        <div>
          <span>Sidecar 控制</span>
          <p>通过环境变量配置 Sidecar 后，可在这里完成启动握手和安全停止。</p>
        </div>
        <div className="button-row">
          <button type="button" disabled={busy || status.sidecar === "ready"} onClick={() => runSidecarCommand("start_sidecar")}>
            {busy ? "处理中…" : "启动 Sidecar"}
          </button>
          <button type="button" className="secondary" disabled={busy || status.sidecar !== "ready"} onClick={() => runSidecarCommand("stop_sidecar")}>
            停止 Sidecar
          </button>
        </div>
      </section>

      <section className="jobs-card" aria-label="Recent archive jobs">
        <div className="section-heading">
          <div>
            <span>最近任务</span>
            <p>SQLite 中由 Rust 管理的归档任务。</p>
          </div>
          <button type="button" className="secondary" onClick={refreshJobs}>刷新</button>
        </div>
        {jobs.length === 0 ? (
          <p className="empty-state">暂无归档任务。通过浏览器或后续任务入口提交 Tweet 后，任务会显示在这里。</p>
        ) : (
          <div className="job-list">
            {jobs.map((job) => (
              <article className="job-row" key={job.job_id}>
                <div>
                  <strong>Tweet {job.tweet_id}</strong>
                  <span>{job.job_id} · {job.tweet_type}</span>
                </div>
                <div className="job-meta">
                  <b>{job.state}</b>
                  <span>{job.updated_at}</span>
                  {job.last_error_message && <em>{job.last_error_code || "ERROR"}: {job.last_error_message}</em>}
                </div>
              </article>
            ))}
          </div>
        )}
      </section>

      {status.database_error && <p className="error">SQLite 初始化失败：{status.database_error}</p>}
      {status.sidecar_error && <p className="error">Sidecar 错误：{status.sidecar_error}</p>}
      {error && <p className="error">Tauri command 暂不可用：{error}</p>}
      <footer className="footer">Rust 是唯一业务状态所有者 · 协议版本 1 · Sidecar 状态由 Rust 管理</footer>
    </main>
  );
}

function StatusCard({ label, value }) {
  return (
    <article className="status-card">
      <span>{label}</span>
      <strong>{value}</strong>
    </article>
  );
}

createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);