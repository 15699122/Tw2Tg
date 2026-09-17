import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Icon from "../components/icon.jsx";
import { Button } from "../components/ui/button";
import { Card, CardContent } from "../components/ui/card";
import { PageHeader, Alert } from "./shared.jsx";
import {
  DEFAULT_LOG_LINE_LIMIT,
  LOG_LEVEL_OPTIONS,
  filterLogEntries,
  parseLogEntries,
} from "../lib/log-lines.js";

export default function LogsPage() {
  const [lines, setLines] = useState([]);
  const [level, setLevel] = useState("info");
  const [query, setQuery] = useState("");
  const [follow, setFollow] = useState(true);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const viewportRef = useRef(null);

  const loadLogs = () =>
    invoke("read_application_logs", { limit: DEFAULT_LOG_LINE_LIMIT })
      .then((next) => {
        setLines(Array.isArray(next) ? next : []);
        setError("");
      })
      .catch((reason) => {
        const message = String(reason || "");
        if (!message.includes("no application log file exists")) {
          setError(`运行日志读取失败：${message}`);
        }
      })
      .finally(() => setLoading(false));

  useEffect(() => {
    loadLogs();
    const timer = window.setInterval(loadLogs, 1000);
    return () => window.clearInterval(timer);
  }, []);

  const entries = useMemo(() => {
    const filtered = filterLogEntries(parseLogEntries(lines), level);
    const normalizedQuery = query.trim().toLowerCase();
    return normalizedQuery
      ? filtered.filter((entry) => entry.raw.toLowerCase().includes(normalizedQuery))
      : filtered;
  }, [lines, level, query]);

  useEffect(() => {
    if (follow && viewportRef.current) {
      viewportRef.current.scrollTop = viewportRef.current.scrollHeight;
    }
  }, [entries, follow]);

  const copyLogs = () => {
    const text = entries.map((entry) => entry.raw).join("\n");
    if (text) invoke("copy_text_to_clipboard", { text }).catch(() => {});
  };

  const handleScroll = () => {
    const element = viewportRef.current;
    if (!element) return;
    const pinned = element.scrollHeight - element.scrollTop - element.clientHeight <= 32;
    setFollow(pinned);
  };

  return (
    <>
      <PageHeader
        eyebrow="XARCHIVE / RUNTIME LOG"
        title="运行日志"
        description="实时查看应用、数据库、Sidecar 和下载流程的诊断信息。"
        action={
          <div className="topbar-actions">
            <Button variant="outline" size="sm" onClick={loadLogs}>
              <Icon name="refresh" size={14} />刷新
            </Button>
            <Button variant="outline" size="sm" onClick={copyLogs} disabled={!entries.length}>
              <Icon name="copy" size={14} />复制可见日志
            </Button>
            <Button variant="outline" size="sm" onClick={() => invoke("open_log_folder").catch(() => {})}>
              <Icon name="folder" size={14} />打开日志目录
            </Button>
          </div>
        }
      />
      {error && <Alert message={error} />}
      <Card className="logs-panel">
        <CardContent>
          <div className="logs-toolbar">
            <label htmlFor="log-level">最低等级</label>
            <select id="log-level" value={level} onChange={(event) => setLevel(event.target.value)}>
              {LOG_LEVEL_OPTIONS.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
            </select>
            <label className="logs-search-label" htmlFor="log-search">搜索</label>
            <input id="log-search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索日志内容" />
            <label className="logs-follow"><input type="checkbox" checked={follow} onChange={(event) => setFollow(event.target.checked)} />自动跟随</label>
          </div>
          <div className="log-viewport" ref={viewportRef} onScroll={handleScroll} role="log" aria-live="polite" aria-label="运行日志内容">
            {loading ? <p className="log-empty">正在加载日志…</p> : entries.length ? entries.map((entry) => <div className={`log-line log-level-${entry.level || "unknown"}`} key={`${entry.index}-${entry.raw}`}>{entry.raw}</div>) : <p className="log-empty">暂无匹配的日志记录。</p>}
          </div>
          {!follow && <Button className="logs-bottom-button" size="sm" onClick={() => { setFollow(true); if (viewportRef.current) viewportRef.current.scrollTop = viewportRef.current.scrollHeight; }}>回到底部</Button>}
        </CardContent>
      </Card>
    </>
  );
}