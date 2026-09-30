export function displayFileName(fullPath) {
  if (!fullPath) {
    return "未检测到路径";
  }
  const normalized = String(fullPath).replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);
  return parts.length ? parts[parts.length - 1] : String(fullPath);
}

export function extensionSidebarState({ filesReady, browserConnection, nativeHost, initialLoad, checking = false }) {
  if (initialLoad) {
    return { tone: "muted", text: "检测中…" };
  }
  if (checking || browserConnection === "checking") {
    return { tone: "muted", text: "检测中…" };
  }
  if (browserConnection === "connected") {
    return { tone: "online", text: "已连接" };
  }
  if (!filesReady) {
    return { tone: "error", text: "文件缺失" };
  }
  if (nativeHost === "not_registered") {
    return { tone: "error", text: "Host 未注册" };
  }
  if (browserConnection === "not_loaded" || browserConnection === "disconnected") {
    return { tone: "error", text: "未连接" };
  }
  if (browserConnection === "error" || nativeHost === "error") {
    return { tone: "error", text: "检测失败" };
  }
  return { tone: "error", text: "未连接" };
}

export function aria2StatusText(installation) {
  if (!installation) {
    return "当前程序未找到 aria2c";
  }
  if (installation.found && installation.version) {
    return `v${installation.version}`;
  }
  if (installation.found) {
    return "已检测到 aria2c";
  }
  return "当前程序未找到 aria2c";
}

export const PROXY_MODES = [
  { value: "system", label: "跟随系统", detail: "使用操作系统或环境中的代理设置。" },
  { value: "direct", label: "直连", detail: "始终不使用代理，并清除继承的代理环境变量。" },
  { value: "manual", label: "手动", detail: "使用下方填写的代理地址。" },
];

/**
 * The label and detail for the selected mode.
 *
 * An unknown mode falls back to `system` instead of rendering an empty row, so
 * a configuration written by a newer build still produces a usable control.
 */
export function proxyModeOption(value) {
  return (
    PROXY_MODES.find((mode) => mode.value === value) ||
    PROXY_MODES.find((mode) => mode.value === "system")
  );
}

/**
 * The status line describing what the current configuration will actually do.
 *
 * This deliberately distinguishes a stored value from a value in effect. Showing
 * "已配置" for a value that the selected mode ignores would tell the user their
 * proxy is protecting traffic when it is not.
 */
export function proxyStatusText(settings) {
  if (!settings || !settings.proxy_mode) {
    return "尚未读取代理设置。";
  }
  const mode = proxyModeOption(settings.proxy_mode);
  if (settings.proxy_mode === "manual") {
    if (!settings.proxy_configured) {
      return { tone: "warning", text: "手动模式需要填写代理地址后才能保存。" };
    }
    return { tone: "online", text: `正在使用 ${settings.proxy_summary || "已配置的代理"}。` };
  }
  if (settings.proxy_configured && !settings.proxy_active) {
    return {
      tone: "muted",
      text: `已保存代理地址，但${mode.label}模式不会使用它；切回手动模式即可继续使用。`,
    };
  }
  if (settings.proxy_mode === "direct") {
    return { tone: "online", text: "所有出站请求直连，aria2 与 Sidecar 的代理变量会被清除。" };
  }
  return { tone: "online", text: mode.detail };
}

/**
 * The coverage boundary shown under the control.
 *
 * The UI must not promise per-URL PAC results the resolver has not computed, so
 * the note comes from the backend rather than being hard-coded here.
 */
export function proxyCoverageText(settings) {
  if (!settings || !settings.system_proxy_note) {
    return "";
  }
  const suffix = settings.system_proxy_supported
    ? "支持按 URL 解析 PAC 与 WPAD。"
    : "暂不支持 PAC/WPAD 的按 URL 解析。";
  return `${settings.system_proxy_note}${suffix}本地 aria2 RPC 与浏览器扩展连接始终直连。`;
}

/**
 * Validate a manual proxy value before it is sent to the backend.
 *
 * The backend validates again; this exists so the user sees the problem without
 * a round trip.
 */
export function validateManualProxy(mode, value) {
  if (mode !== "manual") {
    return "";
  }
  const trimmed = String(value || "").trim();
  if (!trimmed) {
    return "手动模式需要填写代理地址。";
  }
  if (/\s/.test(trimmed)) {
    return "代理地址不能包含空格。";
  }
  if (!/^(https?|socks5h?|ftp):\/\/[^\s@]+(?:@[^\s@]*)?$/i.test(trimmed)) {
    return "代理地址需要形如 http://host:port。";
  }
  return "";
}

