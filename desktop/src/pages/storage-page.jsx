import { Button } from "../components/ui/button";
import CopyablePath from "../components/copyable-path.jsx";
import SettingsSection from "../components/settings-section.jsx";
import { PageHeader, Alert } from "./shared.jsx";

/// 存储页：归档媒体文件的本地保存位置。
/// 该页面预留给后续的归档元数据编辑入口。
export default function StoragePage({
  archiveRoot = "", archiveBusy = false, chooseArchiveDirectory = () => {}, archiveError = "",
  copyPath = () => {}, copied = "", expandedSections = {}, toggleSettingsSection = () => {},
}) {
  return <>
    <PageHeader eyebrow="XARCHIVE / STORAGE" title="存储" description="管理归档媒体文件的本地保存目录。" />
    <div className="settings-layout">
      <SettingsSection id="archive-storage-settings" title="存储位置" description="选择归档媒体文件保存的本地目录。" icon="folder" expanded={expandedSections.storage ?? false} onToggle={() => toggleSettingsSection("storage")}>
        <div className="settings-section-content">{archiveRoot && <CopyablePath label="当前归档目录" value={archiveRoot} copied={copied === "archive"} onCopy={() => copyPath("archive", archiveRoot)} />}<div className="button-row"><Button variant="outline" disabled={archiveBusy} onClick={chooseArchiveDirectory}>{archiveBusy ? "选择中…" : "更改归档目录"}</Button></div>{archiveError && <Alert message={archiveError} />}</div>
      </SettingsSection>
    </div>
  </>;
}
