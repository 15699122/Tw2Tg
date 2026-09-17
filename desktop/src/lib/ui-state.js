export function displayFileName(fullPath) {
  if (!fullPath) {
    return "未检测到路径";
  }
  const normalized = String(fullPath).replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);
  return parts.length ? parts[parts.length - 1] : String(fullPath);
}

export function extensionSidebarState({ filesReady, browserConnection, initialLoad }) {
  if (initialLoad) {
    return { tone: "muted", text: "检测中…" };
  }
  if (browserConnection === "connected") {
    return { tone: "online", text: "已连接" };
  }
  if (!filesReady) {
    return { tone: "error", text: "文件缺失" };
  }
  if (browserConnection === "unknown" || browserConnection === "checking") {
    return { tone: "muted", text: "检测中…" };
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
