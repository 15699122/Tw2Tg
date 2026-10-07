import { Button } from "../components/ui/button";
import Icon from "../components/icon.jsx";
import CopyablePath from "../components/copyable-path.jsx";
import SettingsSection from "../components/settings-section.jsx";
import { Toggle } from "../components/ui/toggle.jsx";
import { Badge } from "../components/ui/badge";
import { aria2StatusText } from "../lib/ui-state.js";
import { PageHeader, Alert } from "./shared.jsx";

/// 下载配置页：媒体传输方式（gallery-dl / aria2）与 aria2 下载引擎管理。
/// 归档存储位置已拆分到「存储」页，任务记录在「任务记录」页。
export default function DownloadConfigPage({
  isWindows, useAria2, useAria2Busy, useAria2Message, saveUseAria2,
  aria2, aria2Busy, aria2CustomPath = "", aria2PathBusy, aria2PathMessage,
  refreshAria2, downloadAria2, checkAria2Path, saveAria2Path, chooseAria2,
  errors = {}, copyPath = () => {}, copied = "", expandedSections = {}, toggleSettingsSection = () => {},
  outputSettings, setOutputSettings, outputSettingsBusy, outputSettingsMessage, saveOutputSettings,
}) {
  return <>
    <PageHeader eyebrow="XARCHIVE / DOWNLOAD CONFIG" title="下载配置" description="选择媒体下载方式并管理 aria2 下载引擎。" />
    <SettingsSection id="output-settings" title="归档输出设置" description="任务在提交时固定使用当前设置；已提交任务不会随全局设置改变。" icon="file" expanded={expandedSections.output ?? true} onToggle={() => toggleSettingsSection("output")}>
      <div className="settings-section-content">
        <div className="settings-fields">
          <div className="settings-field"><label htmlFor="output-naming-mode">媒体命名</label><select id="output-naming-mode" value={outputSettings?.naming_mode || "original"} disabled={outputSettingsBusy} onChange={(event) => setOutputSettings((current) => ({ ...current, naming_mode: event.target.value }))}><option value="original">保留原始文件名</option><option value="template">使用模板命名</option></select></div>
          {outputSettings?.naming_mode === "template" && <div className="settings-field"><label htmlFor="output-filename-template">文件名模板</label><input id="output-filename-template" value={outputSettings.filename_template} maxLength={512} disabled={outputSettingsBusy} onChange={(event) => setOutputSettings((current) => ({ ...current, filename_template: event.target.value }))} placeholder="{username}_{tweet_id}_{index}" /></div>}
        </div>
        <div className="batch-form-options"><Toggle checked={outputSettings?.export_json ?? true} disabled={outputSettingsBusy} onCheckedChange={(value) => setOutputSettings((current) => ({ ...current, export_json: value }))}>导出 tweet.json</Toggle><Toggle checked={outputSettings?.export_text ?? true} disabled={outputSettingsBusy} onCheckedChange={(value) => setOutputSettings((current) => ({ ...current, export_text: value }))}>导出 tweet.txt</Toggle></div>
        <p className="settings-help">模板命名和关闭导出将在独立恢复清单批次完成后生效；目前只保存设置并将快照固定到新任务/批次，实际归档仍保持原名称和默认导出。</p>
        {outputSettingsMessage && <p className="settings-message" role="status">{outputSettingsMessage}</p>}
        <Button size="sm" disabled={outputSettingsBusy} onClick={saveOutputSettings}>{outputSettingsBusy ? "保存中…" : "保存输出设置"}</Button>
      </div>
    </SettingsSection>
    <section id="download-settings" className="download-settings-section" aria-labelledby="download-settings-title">
      <h2 id="download-settings-title" className="download-section-heading">下载设置</h2>
      <div className="settings-layout">
        <SettingsSection id="transfer-backend-settings" title="下载方式" description="选择由 gallery-dl 直接下载，还是由 aria2 传输。" icon="activity" expanded={expandedSections.transfer ?? false} onToggle={() => toggleSettingsSection("transfer")}>
          <div className="settings-section-content"><p className="settings-help">关闭时由 gallery-dl 下载媒体；开启时使用 aria2 传输。切换会重建下载执行器及连接配置，可能中断正在运行的任务；请先等待活动任务结束。</p><Toggle className="settings-field" checked={useAria2} disabled={useAria2Busy} onCheckedChange={saveUseAria2}>使用 aria2 传输媒体</Toggle>{useAria2Message && <p className="settings-message" role="status">{useAria2Message}</p>}</div>
        </SettingsSection>
        {isWindows && <Aria2Settings installation={aria2} busy={aria2Busy} pathBusy={aria2PathBusy} error={errors.aria2} customPath={aria2CustomPath} pathMessage={aria2PathMessage} copied={copied} copyPath={copyPath} onRefresh={refreshAria2} onDownload={downloadAria2} onCheck={checkAria2Path} onSavePath={saveAria2Path} onChoose={chooseAria2} expanded={expandedSections.aria2 ?? false} onToggle={() => toggleSettingsSection("aria2")} />}
      </div>
    </section>
  </>;
}

function Aria2Settings({ installation, busy, pathBusy, error, customPath, pathMessage, copied, copyPath, onRefresh, onDownload, onCheck, onSavePath, onChoose, expanded, onToggle }) {
  return <SettingsSection id="aria2-settings" title="aria2" description="管理可选下载引擎及其可执行文件路径。" icon="download" expanded={expanded} onToggle={onToggle} actions={<Button variant="ghost" size="icon" aria-label="检测 aria2" onClick={onRefresh} disabled={busy}><Icon name="refresh" size={18} /></Button>}><div className="settings-section-content">{error && <Alert message={error} />}<div className="aria2-status-row"><Badge variant={installation.found ? "success" : "warning"}>{installation.found ? "已检测到" : "未检测到"}</Badge><span>{aria2StatusText(installation)}</span></div>{installation.path && <CopyablePath label="当前 aria2 可执行文件" value={installation.path} copied={copied === "aria2"} onCopy={() => copyPath("aria2", installation.path)} />}<div className="aria2-path-controls"><span className="aria2-path-label">自定义 aria2 路径</span><div className="aria2-path-actions"><Button variant="outline" size="sm" onClick={onChoose} disabled={pathBusy}>选择文件</Button><Button variant="outline" size="sm" onClick={onCheck} disabled={pathBusy || !customPath.trim()}>校验</Button><Button size="sm" onClick={onSavePath} disabled={pathBusy || !customPath.trim()}>保存</Button><Button className="aria2-download-button" onClick={onDownload} disabled={busy}>{busy ? "正在下载…" : installation.found ? "重新安装最新版" : "下载并安装"}<Icon name="download" size={17} /></Button></div>{customPath && <CopyablePath label="待保存的 aria2 路径" value={customPath} copied={copied === "aria2-custom"} onCopy={() => copyPath("aria2-custom", customPath)} />}{pathMessage && <p className="aria2-help" role="status">{pathMessage}</p>}</div><p className="aria2-help aria2-section-note">仅使用官方 aria2 Windows x64 发布包，下载后会校验 SHA-256；自定义路径必须指向可运行的 aria2c。</p></div></SettingsSection>;
}
