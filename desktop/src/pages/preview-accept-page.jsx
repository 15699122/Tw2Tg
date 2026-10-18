import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { PageHeader } from "./shared.jsx";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/// F3 预览/接受页：对待导入记录做媒体证据预览，再在维护守护下单事务提交。
export default function PreviewAcceptPage({
  recordsJson = "",
  archiveRoot = "",
  setRecordsJson = () => {},
  setArchiveRoot = () => {},
  previewBusy = false,
  previewMessage = "",
  previewResult = null,
  acceptBusy = false,
  acceptMessage = "",
  acceptResult = null,
  previewImport = () => {},
  acceptImport = () => {},
}) {
  const [exportJobId, setExportJobId] = useState("");
  const [exportTweetId, setExportTweetId] = useState("");
  const [exportBatchId, setExportBatchId] = useState("");
  const [exportKind, setExportKind] = useState("download");
  const [exportMessage, setExportMessage] = useState("");
  const [selected, setSelected] = useState([]);
  useEffect(() => {
    setSelected((previewResult?.entries ?? []).flatMap((entry, index) => entry.verdict === "accept" ? [index] : []));
  }, [previewResult]);
  const exportRecords = async (format) => {
    try {
      const document = await invoke("export_record_exchange", { jobId: exportJobId, format, kind: exportKind, archiveRoot, tweetId: exportTweetId, batchId: exportBatchId });
      const url = URL.createObjectURL(new Blob([document], { type: "text/plain;charset=utf-8" }));
      const anchor = window.document.createElement("a");
      anchor.href = url;
      anchor.download = `${exportKind}-records.${format}`;
      anchor.click();
      window.setTimeout(() => URL.revokeObjectURL(url), 0);
      setExportMessage("已生成记录文件。");
    } catch (reason) {
      setExportMessage(`导出失败：${String(reason)}`);
    }
  };
  const entryBadge = (verdict) =>
    verdict === "accept" ? (
      <Badge variant="default">通过</Badge>
    ) : (
      <Badge variant="destructive">保留</Badge>
    );

  return <>
    <PageHeader
      eyebrow="XARCHIVE / RECORD IMPORT"
      title="记录导入"
      description="预览待导入记录的媒体证据，再在维护守护下单事务接受。"
    />
    <div className="settings-layout">
      <section className="settings-section">
        <div className="settings-section-content">
          <div className="settings-fields">
            <div className="settings-field">
              <label htmlFor="record-export-job">导出已归档任务记录（Job ID）</label>
              <input id="record-export-job" value={exportJobId} onChange={(event) => setExportJobId(event.target.value)} />
              <label htmlFor="record-export-tweet">或填写 Tweet ID（包括导入记录）</label>
              <input id="record-export-tweet" value={exportTweetId} onChange={(event) => setExportTweetId(event.target.value)} />
              <label htmlFor="record-export-batch">或填写 Batch ID（导出范围三选一）</label>
              <input id="record-export-batch" value={exportBatchId} onChange={(event) => setExportBatchId(event.target.value)} />
              <label htmlFor="record-export-kind">记录类型</label>
              <select id="record-export-kind" value={exportKind} onChange={(event) => setExportKind(event.target.value)}>
                <option value="download">下载记录</option>
                <option value="archive">归档记录（使用下方归档根目录）</option>
              </select>
              <Button disabled={[exportJobId, exportTweetId, exportBatchId].filter((id) => id.trim()).length !== 1 || (exportKind === "archive" && !archiveRoot.trim())} onClick={() => exportRecords("json")}>导出 JSON</Button>
              <Button disabled={[exportJobId, exportTweetId, exportBatchId].filter((id) => id.trim()).length !== 1 || (exportKind === "archive" && !archiveRoot.trim())} onClick={() => exportRecords("yaml")}>导出 YAML</Button>
              {exportMessage && <p role="status">{exportMessage}</p>}
            </div>
            <div className="settings-field">
              <label htmlFor="record-import-records">导入记录（版本化 JSON / YAML 文档）</label>
              <textarea
                id="record-import-records"
                rows={10}
                value={recordsJson}
                disabled={previewBusy || acceptBusy}
                onChange={(event) => setRecordsJson(event.target.value)}
                placeholder={'{"schema_version":1,"kind":"tw2tg.download-records","records":[]}'} />
              </div>
              <div className="settings-field">
                <label htmlFor="record-import-root">归档根目录</label>
                <input
                  id="record-import-root"
                  value={archiveRoot}
                  disabled={previewBusy || acceptBusy}
                  onChange={(event) => setArchiveRoot(event.target.value)}
                  placeholder="/archive"
                />
              </div>
              {previewMessage && <p className="settings-message" role="status">{previewMessage}</p>}
              {acceptMessage && <p className="settings-message" role="status">{acceptMessage}</p>}
              <div className="batch-form-options">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={previewBusy || acceptBusy || !recordsJson.trim() || !archiveRoot.trim()}
                  onClick={previewImport}
                >
                  {previewBusy ? "预览中…" : "预览导入"}
                </Button>
                <Button
                  size="sm"
                  disabled={
                    acceptBusy || previewBusy || !previewResult || selected.length === 0
                  }
                  onClick={() => acceptImport(selected)}
                >
                  {acceptBusy ? "接受中…" : "接受导入"}
                </Button>
                {acceptBusy && <Button variant="outline" onClick={() => {
                  invoke("cancel_record_import").catch((reason) => setExportMessage(`取消失败：${String(reason)}`));
                }}>取消等待</Button>}
              </div>
            </div>
          </div>
          {previewResult && previewResult.entries.length > 0 && (
            <div className="settings-field">
              <label>预览结果（共 {previewResult.entries.length} 条记录）</label>
              <table className="preview-table">
                <thead>
                  <tr>
                    <th>选择</th>
                    <th>身份</th>
                    <th> verdict </th>
                    <th> 原因 </th>
                  </tr>
                </thead>
                <tbody>
                  {previewResult.entries.map((entry, index) => (
                    <tr key={index}>
                      <td><input type="checkbox" aria-label={`选择 ${entry.identity}`}
                        disabled={entry.verdict !== "accept" || acceptBusy}
                        checked={selected.includes(index)}
                        onChange={(event) => setSelected((previous) => event.target.checked
                          ? [...previous, index] : previous.filter((item) => item !== index))} /></td>
                      <td>{entry.identity}</td>
                      <td>{entryBadge(entry.verdict)}</td>
                      <td>{entry.reason || ""}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          {acceptResult && (
            <div className="settings-field">
              <label>接受结果</label>
              <p className="settings-message" role="status">
                {acceptResult.status === "committed"
                  ? `已提交：${acceptResult.accepted} 条接受，${acceptResult.identical ?? 0} 条相同（无变更）`
                  : acceptResult.status === "conflicts"
                    ? `冲突，未提交：${acceptResult.identities?.join(", ") ?? "未知"}`
                    : `回滚：${acceptResult.failures?.length ?? 0} 条失败`}
              </p>
            </div>
          )}
      </section>
    </div>
  </>;
}

