export function displayFileName(fullPath) {
  if (!fullPath) {
    return "未检测到路径";
  }
  const normalized = String(fullPath).replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);
  return parts.length ? parts[parts.length - 1] : String(fullPath);
}

export function extensionConnectionLabel(connection) {
  const labels = {
    connected: "浏览器已连接",
    disconnected: "未连接",
    not_loaded: "未连接",
    checking: "检测中…",
    unknown: "状态未知",
    error: "检测失败",
  };
  return labels[connection] || "状态未知";
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
  return { tone: "muted", text: mode.detail };
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
  // The backend note is an English sentence that already ends in a full stop.
  // Concatenating it directly against the Chinese clause produced text such as
  // "enabled.暂不支持…", so the clauses are kept apart.
  return [
    settings.system_proxy_note.trim(),
    suffix,
    "本地 aria2 RPC 与浏览器扩展连接始终直连。",
  ].join(" ");
}

/**
 * The resolver backing `System` mode, in plain language.
 *
 * The backend reports which resolver it actually uses, because the environment
 * and the operating system look the same in the UI but are different policies.
 */
export function proxyBackendLabel(backend) {
  if (backend === "windows-os") {
    return "Windows 系统代理（静态 / PAC / WPAD）";
  }
  if (backend === "environment") {
    return "进程环境变量";
  }
  return "未知来源";
}

/**
 * The PAC/WPAD state reported by the platform resolver.
 *
 * "unsupported" is not the same as "disabled": the first means this build cannot
 * evaluate PAC at all, the second means the OS has it turned off.
 */
export function proxyPacStateText(state) {
  const labels = {
    available: "已加载 PAC 脚本",
    disabled: "系统已关闭 PAC",
    unconfigured: "未配置 PAC",
    "not-found": "未发现 PAC",
    unsupported: "当前平台不支持 PAC",
    "error-discovery": "PAC 自动发现失败",
    "error-download": "PAC 脚本下载失败",
  };
  if (!state) {
    return "未报告";
  }
  return labels[state] || `其他状态：${state}`;
}

/**
 * Whether a child process can follow the current proxy policy on its own.
 *
 * "requires-caller-routing" is not a warning about the diagnostic: it means
 * gallery-dl and aria2 cannot evaluate a PAC/WPAD policy, so archiving is
 * refused rather than silently bypassing it.
 */
export function proxyChildCoverageText(coverage) {
  const labels = {
    "inherits-environment": "aria2 与 gallery-dl 会自行继承系统代理环境。",
    "requires-caller-routing":
      "当前 PAC/WPAD 策略按目标主机返回不同路线，aria2 与 gallery-dl 只能读取一次代理设置，因此无法遵循；归档会被拒绝而不会静默直连。请改用手动或直连模式。",
    "not-applicable": "当前为绝对模式（手动或直连），子进程直接遵循该设置。",
  };
  if (!coverage) {
    return "";
  }
  return labels[coverage] || "";
}

/**
 * One summary row for the system proxy configuration.
 *
 * Returns an empty list rather than placeholders so a field the platform cannot
 * report is simply absent instead of shown as "off".
 */
export function systemProxyRows(system) {
  if (!system) {
    return [];
  }
  const rows = [{ label: "解析来源", value: proxyBackendLabel(system.backend) }];
  if (system.auto_detect) {
    rows.push({ label: "自动发现", value: "已开启（WPAD）" });
  }
  if (system.pac_state) {
    rows.push({ label: "PAC 状态", value: proxyPacStateText(system.pac_state) });
  }
  if (system.pac_url) {
    rows.push({ label: "PAC 地址", value: system.pac_url });
  }
  if (Array.isArray(system.static_proxies) && system.static_proxies.length) {
    rows.push({ label: "系统静态代理", value: system.static_proxies.join("，") });
  }
  if (Array.isArray(system.bypass) && system.bypass.length) {
    rows.push({ label: "环境免代理", value: system.bypass.join("，") });
  }
  if (system.system_bypass) {
    rows.push({ label: "系统免代理", value: system.system_bypass });
  }
  rows.push({
    label: "环境变量代理",
    value: system.environment_proxy ? "已设置" : "未设置",
  });
  return rows;
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

