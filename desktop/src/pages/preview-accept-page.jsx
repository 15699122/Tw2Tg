import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../components/ui/card";
import { PageHeader, statusMessageClass } from "./shared.jsx";
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
  const exportDisabled = [exportJobId, exportTweetId, exportBatchId].filter((id) => id.trim()).length !== 1
    || (exportKind === "archive" && !archiveRoot.trim());
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
      description="预览待导入记录的媒体证据，并在维护模式下以单事务提交导入。"
    />
    <div className="settings-layout">
      <Card>
        <CardHeader>
          <CardTitle>导出已归档记录</CardTitle>
          <CardDescription>按 Job、Tweet 或 Batch 三选一导出；JSON 与 YAML 描述同一份数据。</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="settings-fields">
            <div className="settings-field">
              <label htmlFor="record-export-job">Job ID</label>
              <input id="record-export-job" className="mono-input" value={exportJobId} onChange={(event) => setExportJobId(event.target.value)} />
            </div>
            <div className="settings-field">
              <label htmlFor="record-export-tweet">Tweet ID</label>
              <input id="record-export-tweet" className="mono-input" value={exportTweetId} onChange={(event) => setExportTweetId(event.target.value)} />
            </div>
            <div className="settings-field">
              <label htmlFor="record-export-batch">Batch ID</label>
              <input id="record-export-batch" className="mono-input" value={exportBatchId} onChange={(event) => setExportBatchId(event.target.value)} />
            </div>
            <div className="settings-field">
              <label htmlFor="record-export-kind">记录类型</label>
              <select id="record-export-kind" value={exportKind} onChange={(event) => setExportKind(event.target.value)}>
                <option value="download">下载记录</option>
                <option value="archive">归档记录（使用下方归档根目录）</option>
              </select>
            </div>
          </div>
          <p className="settings-help">三个范围只能填写一个；选择归档记录时使用导入面板中的归档根目录。</p>
          <div className="button-row">
            <Button variant="outline" size="sm" disabled={exportDisabled} onClick={() => exportRecords("json")}>导出 JSON</Button>
            <Button variant="outline" size="sm" disabled={exportDisabled} onClick={() => exportRecords("yaml")}>导出 YAML</Button>
          </div>
          {exportMessage && <p className={statusMessageClass(exportMessage)} role="status">{exportMessage}</p>}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>导入记录</CardTitle>
          <CardDescription>粘贴版本化 JSON / YAML 文档，先预览媒体证据，再提交接受。</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="settings-field">
            <label htmlFor="record-import-records">记录文档</label>
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
              className="mono-input"
              value={archiveRoot}
              disabled={previewBusy || acceptBusy}
              onChange={(event) => setArchiveRoot(event.target.value)}
              placeholder={"例如 /archive 或 D:\\Archive"}
            />
          </div>
          {previewMessage && <p className={statusMessageClass(previewMessage)} role="status">{previewMessage}</p>}
          {acceptMessage && <p className={statusMessageClass(acceptMessage)} role="status">{acceptMessage}</p>}
          <div className="button-row">
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
              disabled={acceptBusy || previewBusy || !previewResult || selected.length === 0}
              onClick={() => acceptImport(selected)}
            >
              {acceptBusy ? "接受中…" : "接受导入"}
            </Button>
            {acceptBusy && <Button size="sm" variant="outline" onClick={() => {
              invoke("cancel_record_import").catch((reason) => setExportMessage(`取消失败：${String(reason)}`));
            }}>取消等待</Button>}
          </div>
        </CardContent>
      </Card>
      {previewResult && previewResult.entries.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>预览结果</CardTitle>
            <CardDescription>共 {previewResult.entries.length} 条记录；只有通过媒体证据检查的记录可以选中。</CardDescription>
          </CardHeader>
          <CardContent>
            <table className="preview-table">
              <thead>
                <tr>
                  <th>选择</th>
                  <th>身份</th>
                  <th>结果</th>
                  <th>原因</th>
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
                    <td className="preview-identity">{entry.identity}</td>
                    <td>{entryBadge(entry.verdict)}</td>
                    <td>{entry.reason || ""}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </CardContent>
        </Card>
      )}
      {acceptResult && (
        <Card>
          <CardHeader>
            <CardTitle>接受结果</CardTitle>
            <CardDescription>维护守护下单事务提交的最终状态。</CardDescription>
          </CardHeader>
          <CardContent>
            <p className={statusMessageClass(acceptResult.status === "committed" ? "已提交" : "未提交")} role="status">
              {acceptResult.status === "committed"
                ? `已提交：${acceptResult.accepted} 条接受，${acceptResult.identical ?? 0} 条相同（无变更）`
                : acceptResult.status === "conflicts"
                  ? `冲突，未提交：${acceptResult.identities?.join(", ") ?? "未知"}`
                  : `回滚：${acceptResult.failures?.length ?? 0} 条失败`}
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  </>;
}
