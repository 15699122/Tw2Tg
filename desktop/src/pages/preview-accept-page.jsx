import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { PageHeader } from "./shared.jsx";

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
              <label htmlFor="record-import-records">导入记录（JSON 数组）</label>
              <textarea
                id="record-import-records"
                rows={10}
                value={recordsJson}
                disabled={previewBusy || acceptBusy}
                onChange={(event) => setRecordsJson(event.target.value)}
                placeholder="[{&quot;" />
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
                    acceptBusy || previewBusy || !previewResult || previewResult.entries.length === 0
                  }
                  onClick={acceptImport}
                >
                  {acceptBusy ? "接受中…" : "接受导入"}
                </Button>
              </div>
            </div>
          </div>
          {previewResult && previewResult.entries.length > 0 && (
            <div className="settings-field">
              <label>预览结果（共 {previewResult.entries.length} 条记录）</label>
              <table className="preview-table">
                <thead>
                  <tr>
                    <th>身份</th>
                    <th> verdict </th>
                    <th> 原因 </th>
                  </tr>
                </thead>
                <tbody>
                  {previewResult.entries.map((entry, index) => (
                    <tr key={index}>
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

