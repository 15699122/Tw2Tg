import { useEffect } from "react";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import Icon from "../components/icon.jsx";
import CopyablePath from "../components/copyable-path.jsx";
import SettingsSection from "../components/settings-section.jsx";
import { Toggle } from "../components/ui/toggle.jsx";
import { aria2StatusText } from "../lib/ui-state.js";
import { PageHeader, Alert, EmptyJobs, LoadingJobs, formatTime, statusLabels } from "./shared.jsx";

export default function DownloadSettingsPage({
  jobs = [], totalJobs = 0, jobsLoading = false, jobsError = "", jobsPage = 0,
  setJobsPage = () => {}, refreshDownloadPage = () => {}, jobDetails = {}, focusedJobId = "", clearFocusedJob = () => {},
  isWindows, useAria2, useAria2Busy, useAria2Message, saveUseAria2,
  aria2, aria2Busy, aria2CustomPath, aria2PathBusy, aria2PathMessage,
  refreshAria2, downloadAria2, checkAria2Path, saveAria2Path, chooseAria2,
  archiveRoot = "", archiveBusy = false, chooseArchiveDirectory = () => {}, archiveError = "",
  errors, copyPath, copied, expandedSections, toggleSettingsSection,
}) {
  const pageCount = Math.max(1, Math.ceil(totalJobs / 20));
  useEffect(() => { if (focusedJobId) document.getElementById(`job-detail-${CSS.escape(focusedJobId)}`)?.scrollIntoView({ block: "nearest" }); }, [focusedJobId, jobs]);
  return <>
    <PageHeader eyebrow="XARCHIVE / DOWNLOADS" title="内容下载" description="查看下载记录、存储位置及当前媒体下载方式。" />
    <section className="jobs-panel downloads-history" aria-label="所有下载任务">
      <header className="section-header"><div><h2>所有下载任务</h2><p>按添加时间倒序 · 共 {totalJobs} 条</p></div><Button variant="ghost" size="sm" onClick={refreshDownloadPage}><Icon name="refresh" size={14} />刷新</Button></header>
      {jobsError ? <Alert message={jobsError} /> : jobsLoading ? <LoadingJobs /> : jobs.length ? <ul className="job-list">{jobs.map((job) => {
        const detail = jobDetails[job.job_id];
        const average = detail?.downloaded_bytes != null && detail?.download_duration_ms > 0 ? `${(detail.downloaded_bytes * 1000 / detail.download_duration_ms / 1024 / 1024).toFixed(2)} MiB/s` : "不适用";
        return <li key={job.job_id} id={`job-detail-${job.job_id}`} className="download-job-entry">
          <div className="download-job-summary"><div><strong>Tweet {job.tweet_id}</strong><small>{job.job_id} · {job.tweet_type}</small></div><Badge variant={job.state === "COMPLETE" ? "success" : job.state === "FAILED" ? "destructive" : job.state === "DOWNLOADING" ? "warning" : "secondary"}>{statusLabels[job.state] || job.state}</Badge></div>
          <dl className="download-job-facts"><div><dt>添加时间</dt><dd>{formatTime(job.created_at)}</dd></div><div><dt>下载完成</dt><dd>{detail?.download_finished_at ? formatTime(detail.download_finished_at) : "未记录"}</dd></div><div><dt>任务用时</dt><dd>{detail?.task_finished_at && detail?.download_started_at ? `${Math.max(0, (new Date(detail.task_finished_at) - new Date(detail.download_started_at)) / 1000).toFixed(1)} 秒` : "未记录"}</dd></div><div><dt>平均下载速度</dt><dd>{average}</dd></div></dl>
          {focusedJobId === job.job_id && <p className="settings-message" role="status">已从最近任务定位 · 下载方式：{detail?.backend || "历史数据未记录"} · 已下载：{detail?.downloaded_bytes == null ? "未记录" : `${detail.downloaded_bytes} B`} · 下载阶段用时：{detail?.download_duration_ms == null ? "未记录" : `${(detail.download_duration_ms / 1000).toFixed(1)} 秒`}{job.last_error_code ? ` · ${job.last_error_code}` : ""}</p>}
          {focusedJobId === job.job_id && <Button variant="ghost" size="sm" onClick={clearFocusedJob}>收起详情</Button>}
        </li>;
      })}</ul> : <EmptyJobs />}
      <div className="button-row"><Button variant="outline" disabled={jobsLoading || jobsPage <= 0} onClick={() => setJobsPage(jobsPage - 1)}>上一页</Button><span aria-live="polite">第 {jobsPage + 1} / {pageCount} 页</span><Button variant="outline" disabled={jobsLoading || jobsPage + 1 >= pageCount} onClick={() => setJobsPage(jobsPage + 1)}>下一页</Button></div>
    </section>
    <div className="settings-layout">
      <SettingsSection id="archive-storage-settings" title="存储位置" description="选择归档媒体文件保存的本地目录。" icon="folder" expanded={expandedSections.storage ?? false} onToggle={() => toggleSettingsSection("storage")}>
        <div className="settings-section-content">{archiveRoot && <CopyablePath label="当前归档目录" value={archiveRoot} copied={copied === "archive"} onCopy={() => copyPath("archive", archiveRoot)} />}<div className="button-row"><Button variant="outline" disabled={archiveBusy} onClick={chooseArchiveDirectory}>{archiveBusy ? "选择中…" : "更改归档目录"}</Button></div>{archiveError && <Alert message={archiveError} />}</div>
      </SettingsSection>
      <SettingsSection id="transfer-backend-settings" title="下载方式" description="选择由 gallery-dl 直接下载，还是由 aria2 传输。" icon="activity" expanded={expandedSections.transfer ?? false} onToggle={() => toggleSettingsSection("transfer")}>
        <div className="settings-section-content"><p className="settings-help">关闭时由 gallery-dl 下载媒体；开启时使用 aria2 传输。配置变更不会主动取消已运行任务。</p><Toggle className="settings-field" checked={useAria2} disabled={useAria2Busy} onCheckedChange={saveUseAria2}>使用 aria2 传输媒体</Toggle>{useAria2Message && <p className="settings-message" role="status">{useAria2Message}</p>}</div>
      </SettingsSection>
      {isWindows && <Aria2Settings installation={aria2} busy={aria2Busy} pathBusy={aria2PathBusy} error={errors.aria2} customPath={aria2CustomPath} pathMessage={aria2PathMessage} copied={copied} copyPath={copyPath} onRefresh={refreshAria2} onDownload={downloadAria2} onCheck={checkAria2Path} onSavePath={saveAria2Path} onChoose={chooseAria2} expanded={expandedSections.aria2 ?? false} onToggle={() => toggleSettingsSection("aria2")} />}
    </div>
  </>;
}

function Aria2Settings({ installation, busy, pathBusy, error, customPath, pathMessage, copied, copyPath, onRefresh, onDownload, onCheck, onSavePath, onChoose, expanded, onToggle }) {
  return <SettingsSection id="aria2-settings" title="aria2" description="管理可选下载引擎及其可执行文件路径。" icon="download" expanded={expanded} onToggle={onToggle} actions={<Button variant="ghost" size="icon" aria-label="检测 aria2" onClick={onRefresh} disabled={busy}><Icon name="refresh" size={18} /></Button>}><div className="settings-section-content">{error && <Alert message={error} />}<div className="aria2-status-row"><Badge variant={installation.found ? "success" : "warning"}>{installation.found ? "已检测到" : "未检测到"}</Badge><span>{aria2StatusText(installation)}</span></div>{installation.path && <CopyablePath label="当前 aria2 可执行文件" value={installation.path} copied={copied === "aria2"} onCopy={() => copyPath("aria2", installation.path)} />}<div className="aria2-path-controls"><span className="aria2-path-label">自定义 aria2 路径</span><div className="aria2-path-actions"><Button variant="outline" size="sm" onClick={onChoose} disabled={pathBusy}>选择文件</Button><Button variant="outline" size="sm" onClick={onCheck} disabled={pathBusy || !customPath.trim()}>校验</Button><Button size="sm" onClick={onSavePath} disabled={pathBusy || !customPath.trim()}>保存</Button><Button className="aria2-download-button" onClick={onDownload} disabled={busy}>{busy ? "正在下载…" : installation.found ? "重新安装最新版" : "下载并安装"}<Icon name="download" size={17} /></Button></div>{customPath && <CopyablePath label="待保存的 aria2 路径" value={customPath} copied={copied === "aria2-custom"} onCopy={() => copyPath("aria2-custom", customPath)} />}{pathMessage && <p className="aria2-help" role="status">{pathMessage}</p>}</div><p className="aria2-help aria2-section-note">仅使用官方 aria2 Windows x64 发布包，下载后会校验 SHA-256；自定义路径必须指向可运行的 aria2c。</p></div></SettingsSection>;
}