import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../components/ui/card";
import Icon from "../components/icon.jsx";
import CopyablePath from "../components/copyable-path.jsx";
import { aria2StatusText } from "../lib/ui-state.js";
import { PageHeader, Alert, StatusRow, PathDisplay } from "./shared.jsx";
import { ComponentBootstrapStatus } from "../components/connection-status.jsx";

const EXTENSION_URL = "https://github.com/Un1Gfn/Tw2Tg/tree/main/extension";

export default function SettingsPage({
  status, errors, sidecarReady, busy, runSidecar, extension, extensionBusy, refreshExtension, bootstrap,
  registerNativeHost, unregisterNativeHost,
  isWindows, aria2, aria2Busy, aria2CustomPath,
  aria2PathBusy, aria2PathMessage, refreshAria2, downloadAria2, checkAria2Path,
  saveAria2Path, galleryDlPath, galleryDlMessage, galleryDlBusy,
  chooseGalleryDl, chooseAria2,
  copyPath, copied, loggingLevel, setLoggingLevel, maxLogFiles, setMaxLogFiles,
  settingsBusy, settingsMessage, saveSettings, folderBusy, openFolder,
}) {
  return (
    <>
      <PageHeader eyebrow="XARCHIVE / SETTINGS" title="设置" description="管理归档位置、运行组件、日志和浏览器连接。" action={<Button variant="outline" size="sm" onClick={refreshExtension} disabled={extensionBusy}><Icon name="refresh" size={14} />{extensionBusy ? "检测中…" : "重新检测"}</Button>} />
      <div className="settings-layout">
        <section className="settings-section" id="bootstrap-settings" tabIndex="-1"><div className="settings-section-header"><CardTitle>Core Bootstrap</CardTitle><CardDescription>组件目录只激活经过固定 catalog 校验的本地版本。</CardDescription></div><div className="settings-section-content"><ComponentBootstrapStatus bootstrap={bootstrap} /></div></section>
        <section className="settings-section" id="sidecar-settings" tabIndex="-1"><div className="settings-section-header"><CardTitle>Sidecar 配置</CardTitle><CardDescription>Sidecar worker 负责 JSONL 协议；gallery-dl 负责实际媒体提取和下载。</CardDescription></div><div className="settings-section-content">
          {errors.sidecar && <Alert message={errors.sidecar} />}
          <StatusRow icon={sidecarReady ? "check" : "activity"} label={sidecarReady ? "Sidecar 正在运行" : "Sidecar 未启动"} detail={sidecarReady ? "已完成 hello → ready 握手" : "当前未检测到可用的运行进程"} ready={sidecarReady} />
          <div className="button-row"><Button disabled={busy || sidecarReady} onClick={() => runSidecar("start_sidecar")}><Icon name="play" size={14} />启动 Sidecar</Button><Button variant="outline" disabled={busy || !sidecarReady} onClick={() => runSidecar("stop_sidecar")}><Icon name="stop" size={14} />停止</Button></div>
          {galleryDlPath && !galleryDlMessage.includes("请重新选择") ? <CopyablePath label="gallery-dl 可执行文件" value={galleryDlPath} copied={copied === "sidecar"} onCopy={() => copyPath("sidecar", galleryDlPath)} /> : <div className="dependency-missing"><span>未检测到 gallery-dl 可执行文件</span><Button variant="outline" size="sm" onClick={chooseGalleryDl} disabled={galleryDlBusy}>{galleryDlBusy ? "校验中…" : "选择文件"}</Button></div>}
          {galleryDlMessage && <p className={`settings-message ${galleryDlMessage.includes("失败") || galleryDlMessage.includes("重新") ? "settings-message-error" : ""}`} role="status">{galleryDlMessage}</p>}
        </div></section>
        {isWindows && <Aria2Settings installation={aria2} busy={aria2Busy} pathBusy={aria2PathBusy} error={errors.aria2} customPath={aria2CustomPath} pathMessage={aria2PathMessage} copied={copied} copyPath={copyPath} onRefresh={refreshAria2} onDownload={downloadAria2} onCheck={checkAria2Path} onSavePath={saveAria2Path} onChoose={chooseAria2} />}
        <section className="settings-section" id="extension-settings" tabIndex="-1"><div className="settings-section-header section-header"><div><CardTitle>浏览器 Extension</CardTitle><CardDescription>Full Package 会预置 Extension；Core Package 请从 GitHub 下载后按浏览器指南加载。</CardDescription></div><Button variant="ghost" size="icon" aria-label="刷新 Extension 状态" onClick={refreshExtension} disabled={extensionBusy}><Icon name="refresh" size={18} /></Button></div><div className="settings-section-content">
          <StatusRow icon="browser" label={extension.files_ready ? "Extension 文件已就绪" : "未找到完整 Extension"} detail={extensionBusy ? "正在重新检测 Extension、Native Host 和浏览器状态。" : extension.message} ready={extension.files_ready && extension.browser_connection === "connected"} />
          {errors.extension && <Alert message={errors.extension} />}<CopyablePath label="Extension 目录" value={extension.directory} copied={copied === "extension"} onCopy={() => copyPath("extension", extension.directory)} />
          <div className="extension-actions"><Button variant="outline" size="sm" onClick={() => window.open(EXTENSION_URL, "_blank", "noopener,noreferrer")}><Icon name="browser" size={14} />打开 GitHub Extension 目录</Button>{isWindows && <><Button size="sm" disabled={extensionBusy || extension.native_host === "missing"} onClick={registerNativeHost}>{extensionBusy ? "处理中…" : "注册 / 修复 Native Host"}</Button><Button variant="outline" size="sm" disabled={extensionBusy || extension.native_host !== "registered"} onClick={unregisterNativeHost}>取消注册</Button></>}</div><ExtensionGuide />
        </div></section>
        <div className="settings-layout-secondary">
          <section className="settings-section" id="storage-settings" tabIndex="-1"><div className="settings-section-header"><CardTitle>存储位置</CardTitle><CardDescription>文件会先经过 staging 校验，再提交到归档目录。</CardDescription></div><div className="settings-section-content storage-content"><CopyablePath label="归档目录" value={status.archive_root} copied={copied === "archive"} onCopy={() => copyPath("archive", status.archive_root)} /><Button variant="outline" size="sm" disabled={folderBusy} onClick={() => openFolder("open_archive_folder", "folder", "归档文件夹打开失败")}><Icon name="folder" size={14} />打开归档文件夹</Button></div></section>
          <section className="settings-section" id="logging-settings" tabIndex="-1"><div className="settings-section-header"><CardTitle>日志设置</CardTitle><CardDescription>日志位于应用程序旁的 logs 文件夹。</CardDescription></div><div className="settings-section-content logging-content"><CopyablePath label="日志目录" value={status.logs_root} copied={copied === "logs"} onCopy={() => copyPath("logs", status.logs_root)} /><div className="settings-fields"><div className="settings-field"><label htmlFor="logging-level">日志等级</label><select id="logging-level" value={loggingLevel} onChange={(event) => setLoggingLevel(event.target.value)}><option value="error">Error</option><option value="warning">Warning</option><option value="info">Info</option><option value="debug">Debug</option><option value="silent">Silent</option></select></div><div className="settings-field"><label htmlFor="max-log-files">最大日志文件数</label><input id="max-log-files" type="number" min="1" max="100" value={maxLogFiles} onChange={(event) => setMaxLogFiles(event.target.value)} /></div><Button className="settings-save-button" variant="outline" size="sm" disabled={settingsBusy} onClick={saveSettings}>{settingsBusy ? "保存中…" : "保存日志设置"}</Button>{settingsMessage && <span className="settings-message" role="status">{settingsMessage}</span>}</div></div></section>
        </div>
      </div>
    </>
  );
}

function Aria2Settings({ installation, busy, pathBusy, error, customPath, pathMessage, copied, copyPath, onRefresh, onDownload, onCheck, onSavePath, onChoose }) {
  return <section className="settings-section" id="aria2-settings" tabIndex="-1"><div className="settings-section-header section-header"><div className="title-with-icon"><span className="component-icon aria2-icon" aria-hidden="true"><Icon name="download" size={20} /></span><div><CardTitle>aria2</CardTitle><CardDescription>可选下载引擎；自动安装受信任的最新官方版本。</CardDescription></div></div><Button variant="ghost" size="icon" aria-label="检测 aria2" onClick={onRefresh} disabled={busy}><Icon name="refresh" size={18} /></Button></div><div className="settings-section-content">{error && <Alert message={error} />}<div className="aria2-status-row"><Badge variant={installation.found ? "success" : "warning"}>{installation.found ? "已检测到" : "未检测到"}</Badge><span>{aria2StatusText(installation)}</span></div>{installation.path && <CopyablePath label="当前 aria2 可执行文件" value={installation.path} copied={copied === "aria2"} onCopy={() => copyPath("aria2", installation.path)} />}<div className="aria2-path-controls"><span className="aria2-path-label">自定义 aria2 路径</span><div className="aria2-path-actions"><Button variant="outline" size="sm" onClick={onChoose} disabled={pathBusy}>选择文件</Button><Button variant="outline" size="sm" onClick={onCheck} disabled={pathBusy || !customPath.trim()}>校验</Button><Button size="sm" onClick={onSavePath} disabled={pathBusy || !customPath.trim()}>保存</Button><Button className="aria2-download-button" onClick={onDownload} disabled={busy}>{busy ? "正在下载…" : installation.found ? "重新安装最新版" : "下载并安装"}<Icon name="download" size={17} /></Button></div>{customPath && <CopyablePath label="待保存的 aria2 路径" value={customPath} copied={copied === "aria2-custom"} onCopy={() => copyPath("aria2-custom", customPath)} />}{pathMessage && <p className="aria2-help" role="status">{pathMessage}</p>}</div><p className="aria2-help">仅使用官方 aria2 Windows x64 发布包，下载后会校验 SHA-256；自定义路径必须指向可运行的 aria2c。</p></div></section>;
}

function ExtensionGuide() {
  return <details className="extension-guide"><summary><Icon name="info" size={16} />未检测到浏览器连接？查看加载步骤</summary><div className="guide-grid"><div><h4>Microsoft Edge</h4><ol><li>打开 <code>edge://extensions</code>。</li><li>开启“开发人员模式”。</li><li>点击“加载解压缩的扩展”。</li><li>选择 XArchive 目录中的 <code>extension</code> 文件夹。</li><li>确认扩展已启用，然后打开或刷新 <code>https://x.com/</code>。</li><li>返回 XArchive，点击“重新检测”。</li></ol></div><div><h4>Google Chrome</h4><ol><li>打开 <code>chrome://extensions</code>。</li><li>开启“开发者模式”。</li><li>点击“加载已解压的扩展程序”。</li><li>选择 XArchive 目录中的 <code>extension</code> 文件夹。</li><li>确认扩展已启用，然后打开或刷新 <code>https://x.com/</code>。</li><li>返回 XArchive，点击“重新检测”。</li></ol></div></div></details>;
}
