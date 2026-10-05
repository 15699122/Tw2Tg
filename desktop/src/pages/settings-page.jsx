import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../components/ui/card";
import Icon from "../components/icon.jsx";
import CopyablePath from "../components/copyable-path.jsx";
import TelegramSettings from "../components/telegram-settings.jsx";
import SettingsSection from "../components/settings-section.jsx";
import { aria2StatusText, PROXY_MODES, proxyCoverageText, proxyModeOption, proxyStatusText, validateManualProxy } from "../lib/ui-state.js";
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
  archiveBusy, archiveMessage, chooseArchiveDirectory,
  proxySettings, proxyValue, setProxyValue, setProxyMode, proxyBusy, proxyMessage, saveProxy, inspectProxy, proxyRoute,
  expandedSections, toggleSettingsSection,
}) {
  return (
    <>
      <PageHeader eyebrow="XARCHIVE / SETTINGS" title="设置" description="管理归档位置、运行组件、日志和浏览器连接。" action={<Button variant="outline" size="sm" onClick={refreshExtension} disabled={extensionBusy}><Icon name="refresh" size={14} />{extensionBusy ? "检测中…" : "重新检测"}</Button>} />
      <div className="settings-layout">
        <SettingsSection id="bootstrap-settings" title="Core Bootstrap" description="查看本地运行组件的可用状态。" icon="folder" expanded={expandedSections.bootstrap ?? false} onToggle={() => toggleSettingsSection("bootstrap")}>
          <div className="settings-section-content"><ComponentBootstrapStatus bootstrap={bootstrap} /><p className="settings-help">仅启用经过固定 catalog 校验的本地组件版本。</p></div>
        </SettingsSection>
        <SettingsSection id="sidecar-settings" title="Sidecar 配置" description="管理归档处理服务和 gallery-dl 路径。" icon="activity" expanded={expandedSections.sidecar ?? false} onToggle={() => toggleSettingsSection("sidecar")}>
          <div className="settings-section-content">
          <p className="settings-help">Sidecar 使用 JSONL（每行一个 JSON 对象）格式的 v2 命令与事件；协议事件写入标准输出，诊断日志写入标准错误。gallery-dl 仅提取媒体元数据和下载地址，媒体文件由 aria2 下载。</p>
          {errors.sidecar && <Alert message={errors.sidecar} />}
          <StatusRow icon={sidecarReady ? "check" : "activity"} showIcon={false} label={sidecarReady ? "Sidecar 正在运行" : "Sidecar 未启动"} detail={sidecarReady ? "已完成 hello → ready 握手" : "当前未检测到可用的运行进程"} ready={sidecarReady} />
          <div className="button-row"><Button disabled={busy || sidecarReady} onClick={() => runSidecar("start_sidecar")}><Icon name="play" size={14} />启动 Sidecar</Button><Button variant="outline" disabled={busy || !sidecarReady} onClick={() => runSidecar("stop_sidecar")}><Icon name="stop" size={14} />停止</Button></div>
          {galleryDlPath && !galleryDlMessage.includes("请重新选择") ? <CopyablePath label="gallery-dl 可执行文件" value={galleryDlPath} copied={copied === "sidecar"} onCopy={() => copyPath("sidecar", galleryDlPath)} /> : <div className="dependency-missing"><span>未检测到 gallery-dl 可执行文件</span><Button variant="outline" size="sm" onClick={chooseGalleryDl} disabled={galleryDlBusy}>{galleryDlBusy ? "校验中…" : "选择文件"}</Button></div>}
          {galleryDlMessage && <p className={`settings-message ${galleryDlMessage.includes("失败") || galleryDlMessage.includes("重新") ? "settings-message-error" : ""}`} role="status">{galleryDlMessage}</p>}
          </div>
        </SettingsSection>
        {isWindows && <Aria2Settings installation={aria2} busy={aria2Busy} pathBusy={aria2PathBusy} error={errors.aria2} customPath={aria2CustomPath} pathMessage={aria2PathMessage} copied={copied} copyPath={copyPath} onRefresh={refreshAria2} onDownload={downloadAria2} onCheck={checkAria2Path} onSavePath={saveAria2Path} onChoose={chooseAria2} expanded={expandedSections.aria2 ?? false} onToggle={() => toggleSettingsSection("aria2")} />}
        <SettingsSection id="extension-settings" title="浏览器 Extension" description="查看浏览器扩展文件与连接状态，管理连接配置。" icon="extension" expanded={expandedSections.extension ?? false} onToggle={() => toggleSettingsSection("extension")} actions={<Button variant="ghost" size="icon" aria-label="刷新 Extension 状态" onClick={refreshExtension} disabled={extensionBusy}><Icon name="refresh" size={18} /></Button>}>
          <div className="settings-section-content">
          <p className="settings-help">Full Package 会预置 Extension；Core Package 用户可从 GitHub 下载扩展并按下方浏览器指南加载。</p>
          <StatusRow icon="browser" showIcon={false} label={extension.files_ready ? "Extension 文件已就绪" : "未找到完整 Extension"} detail={extensionBusy ? "正在重新检测 Extension、Native Host 和浏览器状态。" : extension.message} ready={extension.files_ready && extension.browser_connection === "connected"} />
          {errors.extension && <Alert message={errors.extension} />}<CopyablePath label="Extension 目录" value={extension.directory} copied={copied === "extension"} onCopy={() => copyPath("extension", extension.directory)} />
          <div className="extension-websocket-status"><div><span>WebSocket 端口</span><strong>{extension.websocket_port || "未启动"}</strong></div><div><span>认证状态</span><strong>{extension.websocket_authenticated ? "已认证" : "未认证"}</strong></div><div><span>Desktop 观察</span><strong>{extension.websocket_connection || "未启动"}</strong></div>{extension.websocket_diagnostics && <div className="extension-websocket-diagnostics"><span>连接阶段</span><strong>接受 {extension.websocket_diagnostics.accepted} · 握手失败 {extension.websocket_diagnostics.handshake_failed} · 认证读取失败 {extension.websocket_diagnostics.auth_read_failed} · 收到认证 {extension.websocket_diagnostics.auth_received} · 成功 {extension.websocket_diagnostics.auth_succeeded} · 失败 {extension.websocket_diagnostics.auth_failed} · 认证响应失败 {extension.websocket_diagnostics.auth_response_failed} · 认证前关闭 {extension.websocket_diagnostics.close_before_auth} · 认证后关闭 {extension.websocket_diagnostics.close_after_auth}</strong></div>}{extension.websocket_error && <p className="settings-message settings-message-error" role="status">WebSocket：{extension.websocket_error}</p>}{extension.websocket_token && <CopyablePath label="Extension 配对 token（复制到 Extension 设置）" value={extension.websocket_token} copied={copied === "websocket-token"} onCopy={() => copyPath("websocket-token", extension.websocket_token)} />}</div>
          <div className="extension-actions"><Button variant="outline" size="sm" onClick={() => window.open(EXTENSION_URL, "_blank", "noopener,noreferrer")}><Icon name="extension" size={14} />打开 GitHub Extension 目录</Button>{isWindows && <><Button size="sm" disabled={extensionBusy || extension.native_host === "missing"} onClick={registerNativeHost}>{extensionBusy ? "处理中…" : "注册 / 修复 Native Host"}</Button><Button variant="outline" size="sm" disabled={extensionBusy || extension.native_host !== "registered"} onClick={unregisterNativeHost}>取消注册</Button></>}</div><ExtensionGuide />
          </div>
        </SettingsSection>
        <div className="settings-layout-secondary">
          <TelegramSettings expanded={expandedSections.telegram ?? false} onToggle={() => toggleSettingsSection("telegram")} />
          <SettingsSection id="storage-settings" title="存储位置" description="设置后续归档的保存目录，查看本地数据位置。" icon="archive" expanded={expandedSections.storage ?? false} onToggle={() => toggleSettingsSection("storage")}>
            <p className="settings-help">文件先经过 staging 校验，再提交到归档目录。更改目录仅影响后续归档，不会移动已有文件；数据库和日志位置保持不变。</p>
            <div className="settings-section-content storage-content"><CopyablePath label="归档目录" value={status.archive_root} copied={copied === "archive"} onCopy={() => copyPath("archive", status.archive_root)} /><div className="storage-actions"><Button variant="outline" size="sm" disabled={archiveBusy} onClick={chooseArchiveDirectory}>{archiveBusy ? "正在应用…" : "更改归档目录"}</Button><Button variant="outline" size="sm" disabled={folderBusy} onClick={() => openFolder("open_archive_folder", "folder", "归档文件夹打开失败")}><Icon name="folder" size={14} />打开归档文件夹</Button></div>{archiveMessage && <p className="settings-message" role="status">{archiveMessage}</p>}</div>
          </SettingsSection>
          <SettingsSection id="logging-settings" title="日志设置" description="设置日志级别和保留数量，查看日志目录。" icon="file" expanded={expandedSections.logging ?? false} onToggle={() => toggleSettingsSection("logging")}>
            <p className="settings-help">日志文件保存在应用程序旁的 logs 文件夹中。</p>
            <div className="settings-section-content logging-content"><CopyablePath label="日志目录" value={status.logs_root} copied={copied === "logs"} onCopy={() => copyPath("logs", status.logs_root)} /><div className="settings-fields"><div className="settings-field"><label htmlFor="logging-level">日志等级</label><select id="logging-level" value={loggingLevel} onChange={(event) => setLoggingLevel(event.target.value)}><option value="error">Error</option><option value="warning">Warning</option><option value="info">Info</option><option value="debug">Debug</option><option value="silent">Silent</option></select></div><div className="settings-field"><label htmlFor="max-log-files">最大日志文件数</label><input id="max-log-files" type="number" min="1" max="100" value={maxLogFiles} onChange={(event) => setMaxLogFiles(event.target.value)} /></div><Button className="settings-save-button" variant="outline" size="sm" disabled={settingsBusy} onClick={saveSettings}>{settingsBusy ? "保存中…" : "保存日志设置"}</Button>{settingsMessage && <span className="settings-message" role="status">{settingsMessage}</span>}</div></div>
          </SettingsSection>
          <ProxySettings settings={proxySettings} value={proxyValue} setValue={setProxyValue} setMode={setProxyMode} busy={proxyBusy} message={proxyMessage} onSave={saveProxy} onInspect={inspectProxy} route={proxyRoute} expanded={expandedSections.proxy ?? false} onToggle={() => toggleSettingsSection("proxy")} />
        </div>
      </div>
    </>
  );
}

function ProxySettings({ settings, value, setValue, setMode, busy, message, onSave, onInspect, route, expanded, onToggle }) {
  const selected = settings ? proxyModeOption(settings.proxy_mode) : proxyModeOption("system");
  const status = proxyStatusText(settings);
  const coverage = proxyCoverageText(settings);
  const validation = validateManualProxy(settings ? settings.proxy_mode : "system", value);
  return (
    <SettingsSection id="proxy-settings" title="网络代理" description="配置组件下载、媒体提取和 Telegram 上传的网络代理。" icon="browser" expanded={expanded} onToggle={onToggle}>
      <div className="settings-section-content">
        <div className="settings-fields">
          <div className="settings-field">
            <label htmlFor="proxy-mode">代理模式</label>
            <select id="proxy-mode" value={selected.value} disabled={busy} onChange={(event) => setMode(event.target.value)}>
              {PROXY_MODES.map((mode) => <option key={mode.value} value={mode.value}>{mode.label}</option>)}
            </select>
          </div>
          {selected.value === "manual" && (
            <div className="settings-field">
              <label htmlFor="proxy-value">代理地址</label>
              <input id="proxy-value" type="text" autoComplete="off" spellCheck="false" placeholder="http://proxy.example:8080" value={value} disabled={busy} onChange={(event) => setValue(event.target.value)} aria-describedby="proxy-status" />
            </div>
          )}
        </div>
        <p id="proxy-status" className={`proxy-status proxy-status-${status.tone}`} role="status">{status.text}</p>
        {settings && settings.proxy_configured && !settings.proxy_active && <p className="proxy-help">已保存的代理地址不会被读取或显示，仅用于切换回手动模式。</p>}
        {coverage && <p className="proxy-help">{coverage}</p>}
        {route && <p className={`proxy-help proxy-route-${route.route}`}>{route.url} → {route.message}{route.proxy ? ` （${route.proxy}）` : ""}</p>}
        {validation && <p className="settings-message settings-message-error" role="alert">{validation}</p>}
        {message && <p className={`settings-message ${message.includes("失败") ? "settings-message-error" : ""}`} role="status">{message}</p>}
        <div className="button-row">
          <Button size="sm" disabled={busy || Boolean(validation)} onClick={onSave}>{busy ? "保存中…" : "保存代理设置"}</Button>
          <Button variant="outline" size="sm" disabled={busy} onClick={() => onInspect("https://api.telegram.org")}>检测当前路由</Button>
        </div>
      </div>
    </SettingsSection>
  );
}

function Aria2Settings({ installation, busy, pathBusy, error, customPath, pathMessage, copied, copyPath, onRefresh, onDownload, onCheck, onSavePath, onChoose, expanded, onToggle }) {
  return <SettingsSection id="aria2-settings" title="aria2" description="管理媒体下载引擎及其可执行文件路径。" icon="download" expanded={expanded} onToggle={onToggle} actions={<Button variant="ghost" size="icon" aria-label="检测 aria2" onClick={onRefresh} disabled={busy}><Icon name="refresh" size={18} /></Button>}><div className="settings-section-content">{error && <Alert message={error} />}<div className="aria2-status-row"><Badge variant={installation.found ? "success" : "warning"}>{installation.found ? "已检测到" : "未检测到"}</Badge><span>{aria2StatusText(installation)}</span></div>{installation.path && <CopyablePath label="当前 aria2 可执行文件" value={installation.path} copied={copied === "aria2"} onCopy={() => copyPath("aria2", installation.path)} />}<div className="aria2-path-controls"><span className="aria2-path-label">自定义 aria2 路径</span><div className="aria2-path-actions"><Button variant="outline" size="sm" onClick={onChoose} disabled={pathBusy}>选择文件</Button><Button variant="outline" size="sm" onClick={onCheck} disabled={pathBusy || !customPath.trim()}>校验</Button><Button size="sm" onClick={onSavePath} disabled={pathBusy || !customPath.trim()}>保存</Button><Button className="aria2-download-button" onClick={onDownload} disabled={busy}>{busy ? "正在下载…" : installation.found ? "重新安装最新版" : "下载并安装"}<Icon name="download" size={17} /></Button></div>{customPath && <CopyablePath label="待保存的 aria2 路径" value={customPath} copied={copied === "aria2-custom"} onCopy={() => copyPath("aria2-custom", customPath)} />}{pathMessage && <p className="aria2-help" role="status">{pathMessage}</p>}</div><p className="aria2-help aria2-section-note">仅使用官方 aria2 Windows x64 发布包，下载后会校验 SHA-256；自定义路径必须指向可运行的 aria2c。</p></div></SettingsSection>;
}

function ExtensionGuide() {
  return <details className="extension-guide"><summary><Icon name="info" size={16} />未检测到浏览器连接？查看加载步骤</summary><div className="guide-grid"><div><h4>Microsoft Edge</h4><ol><li>打开 <code>edge://extensions</code>。</li><li>开启“开发人员模式”。</li><li>点击“加载解压缩的扩展”。</li><li>选择 XArchive 目录中的 <code>extension</code> 文件夹。</li><li>确认扩展已启用，然后打开或刷新 <code>https://x.com/</code>。</li><li>返回 XArchive，点击“重新检测”。</li></ol></div><div><h4>Google Chrome</h4><ol><li>打开 <code>chrome://extensions</code>。</li><li>开启“开发者模式”。</li><li>点击“加载已解压的扩展程序”。</li><li>选择 XArchive 目录中的 <code>extension</code> 文件夹。</li><li>确认扩展已启用，然后打开或刷新 <code>https://x.com/</code>。</li><li>返回 XArchive，点击“重新检测”。</li></ol></div></div></details>;
}
