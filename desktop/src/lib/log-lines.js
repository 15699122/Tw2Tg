// 运行日志解析与过滤的纯前端工具。
//
// Rust 侧 `read_application_logs` 返回原始日志行（tracing fmt 输出），
// 解析、分级和过滤全部放在这里，便于用 `node --test` 单独验证，
// 不依赖 Tauri、DOM 或任何运行状态。

const ANSI_PATTERN = /\u001b\[[0-9;]*m/g;

export const LOG_LEVELS = ["error", "warning", "info", "debug", "silent"];

export const LOG_LEVEL_LABELS = {
  error: "Error",
  warning: "Warning",
  info: "Info",
  debug: "Debug",
  silent: "Silent",
};

export const LOG_LEVEL_OPTIONS = [
  { value: "error", label: "Error" },
  { value: "warning", label: "Warning" },
  { value: "info", label: "Info" },
  { value: "debug", label: "Debug" },
  { value: "silent", label: "Silent" },
];

export const DEFAULT_LOG_LEVEL = "info";
export const DEFAULT_LOG_LINE_LIMIT = 500;

// 日志页的“最低等级”只是显示过滤器，而实际写入哪些行由后端 effective level
// 决定。两者必须一致，否则过滤器的默认值会隐藏后端正在写的诊断：预发布渠道
// 的 effective level 是 debug，若过滤器停在 info，页面会把刚写出的模块诊断
// 全部藏起来，用户只能从日志文件里翻。

const LINE_PATTERN =
  /^(\d{10,}|\d{4}-\d{2}-\d{2}T[0-9:.]*(?:Z|[+-]\d{2}:?\d{2})?)\s+([A-Za-z]+)\s+(.*)$/;

export function stripAnsi(line) {
  return String(line ?? "").replace(ANSI_PATTERN, "");
}

export function normalizeLogLevel(value) {
  const level = String(value ?? "").trim().toLowerCase();
  if (level === "warn") return "warning";
  return LOG_LEVELS.includes(level) ? level : "";
}

export function logLevelLabel(level) {
  return LOG_LEVEL_LABELS[normalizeLogLevel(level)] ?? "";
}

// 后端报告的有效等级转成显示过滤器可用的值。
//
// 无法识别或缺失时回退到 `info`：那是一个中性猜测，而拒绝渲染会让日志页
// 空白。一个未知等级不得让过滤器的下拉框脱离 `LOG_LEVEL_OPTIONS`。
export function displayLogLevel(effectiveLevel) {
  return normalizeLogLevel(effectiveLevel) || DEFAULT_LOG_LEVEL;
}

// 判断后端有效等级是否发生了变化。
//
// 用于“后端等级变化时重新同步显示过滤器，但用户已经手动选择后不再覆盖”这条
// 规则：同步只能发生在后端真的改了等级时，否则每秒轮询会把用户的选择反复抹掉。
export function effectiveLevelChanged(previous, next) {
  return displayLogLevel(previous) !== displayLogLevel(next);
}

export function formatLogTimestamp(timestamp) {
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})/.exec(
    String(timestamp ?? ""),
  );
  if (!match) return String(timestamp ?? "");
  return `${match[4]}:${match[5]}:${match[6]}`;
}

export function parseLogLine(line) {
  const raw = stripAnsi(line).replace(/\s+$/, "");
  if (!raw) return null;

  const match = LINE_PATTERN.exec(raw);
  if (!match) {
    return { raw, timestamp: "", level: "", target: "", message: raw };
  }

  const [, timestamp, level, rest] = match;
  const separator = rest.indexOf(": ");
  let target = "";
  let message = rest;
  if (separator > 0) {
    const candidate = rest.slice(0, separator);
    if (candidate && !/\s/.test(candidate)) {
      target = candidate;
      message = rest.slice(separator + 2);
    }
  }

  return { raw, timestamp, level: normalizeLogLevel(level), target, message };
}

export function parseLogEntries(lines) {
  const source = Array.isArray(lines) ? lines : [];
  const entries = [];
  for (let index = 0; index < source.length; index += 1) {
    const entry = parseLogLine(source[index]);
    if (entry) entries.push({ ...entry, index });
  }
  return entries;
}

export function filterLogEntries(entries, level) {
  const list = Array.isArray(entries) ? entries : [];
  const selected = normalizeLogLevel(level) || DEFAULT_LOG_LEVEL;
  if (selected === "silent") return [];
  const minimum = LOG_LEVELS.indexOf(selected);
  return list.filter((entry) => {
    const index = LOG_LEVELS.indexOf(entry.level);
    return index === -1 || index <= minimum;
  });
}

export function isScrollPinned(element, threshold = 32) {
  if (!element) return true;
  return (
    element.scrollHeight - element.scrollTop - element.clientHeight <= threshold
  );
}

export function logLineSummary(lines) {
  const entries = parseLogEntries(lines);
  const counts = { error: 0, warning: 0 };
  for (const entry of entries) {
    if (entry.level === "error") counts.error += 1;
    else if (entry.level === "warning") counts.warning += 1;
  }
  return { total: entries.length, ...counts };
}

export function mergeLogLines(previous, next, limit = DEFAULT_LOG_LINE_LIMIT) {
  const combined = [...(Array.isArray(previous) ? previous : []), ...(Array.isArray(next) ? next : [])];
  const deduplicated = [];
  const seen = new Set();
  for (const line of combined) {
    const value = String(line ?? "");
    if (!value || seen.has(value)) continue;
    seen.add(value);
    deduplicated.push(value);
  }
  return deduplicated.slice(-Math.max(1, limit));
}