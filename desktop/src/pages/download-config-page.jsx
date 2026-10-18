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
  downloaderSettings, setDownloaderSettings, downloaderSettingsBusy, downloaderSettingsMessage, saveDownloaderSettings,
  exchangeDocument = "", setExchangeDocument = () => {}, exchangeBusy = false, exchangeMessage = "", exchangePreview = null,
  exportExchange = () => {}, previewExchange = () => {}, applyExchange = () => {},
}) {
  return <>
    <PageHeader eyebrow="XARCHIVE / DOWNLOAD CONFIG" title="下载配置" description="选择媒体下载方式并管理 aria2 下载引擎。" />
    <SettingsSection id="output-settings" title="归档输出设置" description="任务在提交时固定使用当前设置；已提交任务不会随全局设置改变。" icon="file" expanded={expandedSections.output ?? true} onToggle={() => toggleSettingsSection("output")}>
      <div className="settings-section-content">
        <div className="settings-fields">
          <div className="settings-field"><label htmlFor="output-naming-mode">媒体命名</label><select id="output-naming-mode" value={outputSettings?.naming_mode || "original"} disabled={outputSettingsBusy} onChange={(event) => setOutputSettings((current) => ({ ...current, naming_mode: event.target.value }))}><option value="original">保留原始文件名</option><option value="template">使用模板命名</option></select></div>
          {outputSettings?.naming_mode === "template" && <div className="settings-field"><label htmlFor="output-filename-template">文件名模板</label><input id="output-filename-template" value={outputSettings.filename_template} maxLength={512} disabled={outputSettingsBusy} onChange={(event) => setOutputSettings((current) => ({ ...current, filename_template: event.target.value }))} placeholder="{username}_{tweet_id}_{index}" /></div>}
        </div>
        <div className="batch-form-options"><Toggle checked={outputSettings?.export_json ?? true} disabled={outputSettingsBusy} onCheckedChange={(value) => setOutputSettings((current) => ({ ...current, export_json: value }))}>导出 tweet.json</Toggle><Toggle checked={outputSettings?.export_yaml ?? true} disabled={outputSettingsBusy} onCheckedChange={(value) => setOutputSettings((current) => ({ ...current, export_yaml: value }))}>导出 tweet.yaml</Toggle></div>
        <p className="settings-help">新任务和新批次使用版本化 JSON/YAML 输出快照；已有任务与批次保留原有命名及 JSON/TXT 语义。当前仅已配置隔离执行环境的 Linux 路径开放新执行链路；Windows 仍需平台验证。</p>
        <p className="settings-help" role="note">关闭 tweet.json 和 tweet.yaml 导出不删除数据库中的归档资料。内部恢复清单只保存 Tweet 与媒体的身份、路径、大小和校验值，不复制正文、作者资料或回复/引用关系；提交前会另存任务绑定的业务元数据候选，供中断恢复使用，提交成功时在同一事务中清除该临时副本。</p>
        {outputSettingsMessage && <p className="settings-message" role="status">{outputSettingsMessage}</p>}
        <Button size="sm" disabled={outputSettingsBusy} onClick={saveOutputSettings}>{outputSettingsBusy ? "保存中…" : "保存输出设置"}</Button>
      </div>
    </SettingsSection>
        <SettingsSection id="downloader-arguments" title="下载器参数" description="支持 JSON 字符串数组，按任务快照；当前 gallery-dl 选项尚未审核，暂不可设置。" icon="activity" expanded={expandedSections.downloaderArguments ?? false} onToggle={() => toggleSettingsSection("downloaderArguments")}>
      <div className="settings-section-content">
        <div className="settings-field"><label htmlFor="gallery-dl-arguments">gallery-dl JSON 参数数组（当前策略暂不开放选项）</label><textarea id="gallery-dl-arguments" rows={3} value={downloaderSettings?.gallery_dl_json ?? "[]"} disabled /></div>
        <div className="settings-field"><label htmlFor="aria2-arguments">aria2 JSON 参数数组</label><textarea id="aria2-arguments" rows={3} value={downloaderSettings?.aria2_json ?? "[]"} disabled={downloaderSettingsBusy} onChange={(event) => setDownloaderSettings((current) => ({ ...current, aria2_json: event.target.value }))} /></div>
        <p className="settings-help">参数按独立 argv 元素传递，不经 shell。gallery-dl 当前仅接受空数组；aria2 只接受 Rust 明确审核的选项。参数值不会写入诊断日志。</p>
        {downloaderSettingsMessage && <p className="settings-message" role="status">{downloaderSettingsMessage}</p>}
        <Button size="sm" disabled={downloaderSettingsBusy} onClick={saveDownloaderSettings}>{downloaderSettingsBusy ? "保存中…" : "保存下载器参数"}</Button>
      </div>
    </SettingsSection>
    <SettingsSection id="configuration-exchange" title="配置交换" description="在机器间导出/导入核心配置；不含数据库、缓存、日志路径、代理与凭据。" icon="file" expanded={expandedSections.configurationExchange ?? false} onToggle={() => toggleSettingsSection("configurationExchange")}>
      <div className="settings-section-content">
        <div className="settings-field"><label htmlFor="configuration-exchange-document">配置文档（JSON）</label><textarea id="configuration-exchange-document" rows={8} value={exchangeDocument} disabled={exchangeBusy} onChange={(event) => { setExchangeDocument(event.target.value); }} placeholder="粘贴目标机器导出的配置文档，或先导出本机配置" /></div>
        <p className="settings-help">导出仅包含归档目录、sidecar/gallery-dl/aria2 可执行文件路径、传输后端与日志级别/数量。不含数据库与缓存路径、初始化标记、代理地址、Telegram/扩展设置、凭据或下载器参数。路径以目标机器为准校验：目录须已存在，可执行文件须为常规文件；不匹配的项保留目标机器原值。</p>
        <p className="settings-help" role="note">应用会先关闭新任务准入并等待运行中的任务排空，再原子写入配置；写入失败不会改动当前运行时。需要重启的设置（归档目录、工具路径、传输后端）不会自动重启，重启后生效。</p>
        {exchangePreview && <p className="settings-message" role="status">预览修订：{exchangePreview.revision}{exchangePreview.restart_required ? "；需重启生效" : ""}{exchangePreview.preserved?.length ? `；保留 ${exchangePreview.preserved.length} 项` : ""}</p>}
        {exchangeMessage && <p className="settings-message" role="status">{exchangeMessage}</p>}
        <div className="batch-form-options">
          <Button size="sm" variant="outline" disabled={exchangeBusy} onClick={exportExchange}>导出本机配置</Button>
          <Button size="sm" variant="outline" disabled={exchangeBusy || !exchangeDocument.trim()} onClick={previewExchange}>预览导入</Button>
          <Button size="sm" disabled={exchangeBusy || !exchangePreview} onClick={applyExchange}>{exchangeBusy ? "处理中…" : "应用导入"}</Button>
        </div>
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
