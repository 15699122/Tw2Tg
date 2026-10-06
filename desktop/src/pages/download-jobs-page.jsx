import { useEffect } from "react";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import Icon from "../components/icon.jsx";
import { PageHeader, Alert, EmptyJobs, LoadingJobs, formatTime, statusLabels } from "./shared.jsx";

/// 归档任务记录页：按添加时间倒序展示任务与下载结果。
/// 下载方式、aria2 与存储设置已拆分到各自的独立页面。
export default function DownloadJobsPage({
  jobs = [], totalJobs = 0, jobsLoading = false, jobsError = "", jobsPage = 0,
  setJobsPage = () => {}, refreshDownloadPage = () => {}, jobDetails = {}, focusedJobId = "", clearFocusedJob = () => {},
}) {
  const pageCount = Math.max(1, Math.ceil(totalJobs / 20));
  useEffect(() => { if (focusedJobId) document.getElementById(`job-detail-${CSS.escape(focusedJobId)}`)?.scrollIntoView({ block: "nearest" }); }, [focusedJobId, jobs]);
  return <>
    <PageHeader eyebrow="XARCHIVE / JOBS" title="任务记录" description="按添加时间倒序查看归档任务及其下载结果。" />
    <section id="download-task-history" className="jobs-panel downloads-history" aria-label="所有下载任务" aria-labelledby="download-task-history-title">
      <header className="section-header"><div><h2 id="download-task-history-title">任务记录</h2><p>按添加时间倒序 · 共 {totalJobs} 条</p></div><Button variant="ghost" size="sm" onClick={refreshDownloadPage}><Icon name="refresh" size={14} />刷新</Button></header>
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
      <div className="button-row"><Button variant="outline" disabled={jobsLoading || jobsPage <= 0} onClick={() => setJobsPage(jobsPage - 1)}>上一页</Button><span aria-live="polite">第 {Math.min(jobsPage + 1, pageCount)} / {pageCount} 页</span><Button variant="outline" disabled={jobsLoading || jobsPage + 1 >= pageCount} onClick={() => setJobsPage(jobsPage + 1)}>下一页</Button></div>
    </section>
  </>;
}
