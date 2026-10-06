import { Button } from "../components/ui/button";
import Icon from "../components/icon.jsx";
import CopyablePath from "../components/copyable-path.jsx";
import TelegramSettings from "../components/telegram-settings.jsx";
import SettingsSection from "../components/settings-section.jsx";
import {
  PROXY_MODES, proxyCoverageText, proxyModeOption, proxyStatusText,
  proxyChildCoverageText, systemProxyRows, validateManualProxy,
} from "../lib/ui-state.js";
import { PageHeader, Alert, StatusRow } from "./shared.jsx";
import { ComponentBootstrapStatus } from "../components/connection-status.jsx";

const EXTENSION_URL = "https://github.com/Un1Gfn/Tw2Tg/tree/main/extension";

export default function SettingsPage({
  status, errors, sidecarReady, busy, runSidecar, extension, extensionBusy, refreshExtension, bootstrap,
  registerNativeHost, unregisterNativeHost,
  isWindows,
  galleryDlPath,
  copyPath, copied, loggingLevel, setLoggingLevel, maxLogFiles, setMaxLogFiles,
  settingsBusy, settingsMessage, saveSettings, folderBusy, openFolder,
  proxySettings, proxyValue, setProxyValue, setProxyMode, proxyBusy, proxyMessage, saveProxy, inspectProxy, proxyRoute, proxySystem, proxyDiagnoseUrl, setProxyDiagnoseUrl,
  expandedSections, toggleSettingsSection,
}) {
  return (
    <>
      <PageHeader eyebrow="XARCHIVE / SETTINGS" title="设置" description="管理归档位置、运行组件、日志和浏览器连接。" action={<Button variant="outline" size="sm" onClick={refreshExtension} disabled={extensionBusy}><Icon name="refresh" size={14} />{extensionBusy ? "检测中…" : "重新检测"}</Button>} />
      <div className="settings-layout">
          <SettingsSection id="bootstrap-settings" title="Core Bootstrap" description="查看本地运行组件的可用状态。" icon="folder" expanded={expandedSections.bootstrap ?? false} onToggle={() => toggleSettingsSection("bootstrap")}>
          <div className="settings-section-content bootstrap-content"><ComponentBootstrapStatus bootstrap={bootstrap} /><p className="settings-help">仅启用经过固定 catalog 校验的本地组件版本。</p></div>
        </SettingsSection>
        <SettingsSection id="sidecar-settings" title="Sidecar 配置" description="查看归档处理服务和 gallery-dl 路径。" icon="activity" expanded={expandedSections.sidecar ?? false} onToggle={() => toggleSettingsSection("sidecar")}>
          <div className="settings-section-content">
          <p className="settings-help">Sidecar 使用 JSONL（每行一个 JSON 对象）格式的 v2 命令与事件；协议事件写入标准输出，诊断日志写入标准错误。gallery-dl 默认提取元数据并直接下载媒体文件，媒体传输可选用 aria2（在侧边栏「内容下载」中管理）。</p>
          {errors.sidecar && <Alert message={errors.sidecar} />}
          <StatusRow icon={sidecarReady ? "check" : "activity"} showIcon={false} label={sidecarReady ? "Sidecar 正在运行" : "Sidecar 未启动"} detail={sidecarReady ? "已完成 hello → ready 握手" : "当前未检测到可用的运行进程"} ready={sidecarReady} />
          <div className="button-row"><Button disabled={busy || sidecarReady} onClick={() => runSidecar("start_sidecar")}><Icon name="play" size={14} />启动 Sidecar</Button><Button variant="outline" disabled={busy || !sidecarReady} onClick={() => runSidecar("stop_sidecar")}><Icon name="stop" size={14} />停止</Button></div>
          {galleryDlPath && <CopyablePath label="gallery-dl 可执行文件" value={galleryDlPath} copied={copied === "sidecar"} onCopy={() => copyPath("sidecar", galleryDlPath)} />}
          </div>
        </SettingsSection>
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
          <SettingsSection id="logging-settings" title="日志设置" description="设置日志级别和保留数量，查看日志目录。" icon="file" expanded={expandedSections.logging ?? false} onToggle={() => toggleSettingsSection("logging")}>
            <p className="settings-help">日志文件保存在应用程序旁的 logs 文件夹中。</p>
            <div className="settings-section-content logging-content"><CopyablePath label="日志目录" value={status.logs_root} copied={copied === "logs"} onCopy={() => copyPath("logs", status.logs_root)} /><div className="settings-fields"><div className="settings-field"><label htmlFor="logging-level">日志等级</label><select id="logging-level" value={loggingLevel} onChange={(event) => setLoggingLevel(event.target.value)}><option value="error">Error</option><option value="warning">Warning</option><option value="info">Info</option><option value="debug">Debug</option><option value="silent">Silent</option></select></div><div className="settings-field"><label htmlFor="max-log-files">最大日志文件数</label><input id="max-log-files" type="number" min="1" max="100" value={maxLogFiles} onChange={(event) => setMaxLogFiles(event.target.value)} /></div><Button className="settings-save-button" variant="outline" size="sm" disabled={settingsBusy} onClick={saveSettings}>{settingsBusy ? "保存中…" : "保存日志设置"}</Button>{settingsMessage && <span className="settings-message" role="status">{settingsMessage}</span>}</div></div>
          </SettingsSection>
          <ProxySettings settings={proxySettings} value={proxyValue} setValue={setProxyValue} setMode={setProxyMode} busy={proxyBusy} message={proxyMessage} onSave={saveProxy} onInspect={inspectProxy} route={proxyRoute} system={proxySystem} diagnoseUrl={proxyDiagnoseUrl}
              setDiagnoseUrl={setProxyDiagnoseUrl} expanded={expandedSections.proxy ?? false} onToggle={() => toggleSettingsSection("proxy")} />
        </div>
      </div>
    </>
  );
}

function ProxySettings({ settings, value, setValue, setMode, busy, message, onSave, onInspect, route, system, diagnoseUrl, setDiagnoseUrl, expanded, onToggle }) {
  const selected = settings ? proxyModeOption(settings.proxy_mode) : proxyModeOption("system");
  const status = proxyStatusText(settings);
  const coverage = proxyCoverageText(settings);
  const validation = validateManualProxy(settings ? settings.proxy_mode : "system", value);
  // Only the backend knows which resolver backs `System`, so the summary is
  // derived from its answer rather than from the selected mode.
  const systemRows = systemProxyRows(system);
  const childCoverage = proxyChildCoverageText(system ? system.child_coverage : null);
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
        {systemRows.length > 0 && (
          <details className="proxy-system-details">
            <summary>当前系统代理配置</summary>
            <dl className="proxy-summary">
              {systemRows.map((row) => (
                <div className="proxy-summary-row" key={row.label}>
                  <dt>{row.label}</dt>
                  <dd>{row.value}</dd>
                </div>
              ))}
            </dl>
            <p className="proxy-help">配置变化由系统通知自动感知，解析结果会在配置修订号变化后失效。</p>
            {childCoverage && (
              <p className={`proxy-help proxy-coverage-${childCoverage}`}>{childCoverage}</p>
            )}
          </details>
        )}
        {route && (
          <div className={`proxy-help proxy-route-${route.route}`}>
            <p>{route.url} → {route.message}{route.proxy ? ` （${route.proxy}）` : ""}</p>
            {Array.isArray(route.candidates) && route.candidates.length > 1 && (
              <ol className="proxy-candidate-list">
                {route.candidates.map((candidate, index) => (
                  <li key={`${candidate}-${index}`}>{candidate}</li>
                ))}
              </ol>
            )}
            {Array.isArray(route.candidates) && route.candidates.length === 1 && (
              <p>候选：{route.candidates[0]}（来源：{route.source}）</p>
            )}
          </div>
        )}
        {/* `.settings-field` alone is layout-only: the label and input styling is
            scoped to `.settings-fields`. This field used to sit outside that
            container, so it rendered as an unstyled native control. */}
        <div className="settings-fields proxy-diagnose-fields">
          <div className="settings-field">
            <label htmlFor="proxy-diagnose-url">路由检测地址</label>
            <input id="proxy-diagnose-url" type="text" autoComplete="off" spellCheck="false" placeholder="https://api.telegram.org" value={diagnoseUrl} disabled={busy} onChange={(event) => setDiagnoseUrl(event.target.value)} aria-describedby="proxy-diagnose-help" />
            <p id="proxy-diagnose-help" className="proxy-help">仅解析路由，不会发送请求，也不会显示令牌或代理密码。</p>
          </div>
        </div>
        {validation && <p className="settings-message settings-message-error" role="alert">{validation}</p>}
        {message && <p className={`settings-message ${message.includes("失败") ? "settings-message-error" : ""}`} role="status">{message}</p>}
        <div className="button-row">
          <Button size="sm" disabled={busy || Boolean(validation)} onClick={onSave}>{busy ? "保存中…" : "保存代理设置"}</Button>
          <Button variant="outline" size="sm" disabled={busy} onClick={() => onInspect(diagnoseUrl)}>检测当前路由</Button>
        </div>
      </div>
    </SettingsSection>
  );
}

function ExtensionGuide() {
  return <details className="extension-guide"><summary><Icon name="info" size={16} />未检测到浏览器连接？查看加载步骤</summary><div className="guide-grid"><div><h4>Microsoft Edge</h4><ol><li>打开 <code>edge://extensions</code>。</li><li>开启“开发人员模式”。</li><li>点击“加载解压缩的扩展”。</li><li>选择 XArchive 目录中的 <code>extension</code> 文件夹。</li><li>确认扩展已启用，然后打开或刷新 <code>https://x.com/</code>。</li><li>返回 XArchive，点击“重新检测”。</li></ol></div><div><h4>Google Chrome</h4><ol><li>打开 <code>chrome://extensions</code>。</li><li>开启“开发者模式”。</li><li>点击“加载已解压的扩展程序”。</li><li>选择 XArchive 目录中的 <code>extension</code> 文件夹。</li><li>确认扩展已启用，然后打开或刷新 <code>https://x.com/</code>。</li><li>返回 XArchive，点击“重新检测”。</li></ol></div></div></details>;
}
