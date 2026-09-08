import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./style.css";

const initialStatus = {
  app_name: "XArchive",
  app_version: "0.1.0",
  sidecar: "not_started",
  database: "not_initialized",
  platform: "unknown",
};

function App() {
  const [status, setStatus] = useState(initialStatus);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    invoke("get_app_status")
      .then((value) => {
        if (active) setStatus(value);
      })
      .catch((reason) => {
        if (active) setError(String(reason));
      });
    return () => {
      active = false;
    };
  }, []);

  return (
    <main className="shell">
      <header className="header">
        <div>
          <p className="eyebrow">LOCAL-FIRST ARCHIVE</p>
          <h1>XArchive</h1>
          <p className="subtitle">X/Twitter 本地归档控制台</p>
        </div>
        <span className="badge">DESKTOP SCAFFOLD</span>
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

      {error && <p className="error">Tauri command 暂不可用：{error}</p>}
      <footer className="footer">Rust 是唯一业务状态所有者 · 协议版本 1</footer>
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