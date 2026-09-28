import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Native startup readiness and failure evidence for the Tauri WebDriver suite.
 *
 * A session that never leaves `data:,` is ambiguous on its own: the app may not
 * have navigated, the session may be attached to a blank WebView, or the
 * msedgedriver / WebView2 pairing may be wrong. This module answers the first
 * question directly — it waits until a window target actually holds an
 * XArchive document instead of asserting on a possibly blank target — and
 * writes the artifacts (session-start snapshot, discovery timeline, failure
 * record, screenshots, page source) that the next failure needs to explain
 * itself (WQ-ENG-09b).
 *
 * Contract: best effort. Diagnostics must never replace, mask or change the
 * outcome of the failure that triggered them, and every collection step is
 * guarded so a broken session produces an error entry instead of an exception.
 *
 * `waitForStartupContract` is intentionally absent: this application does not
 * emit the `data-xarchive-startup` marker yet, so a contract gate would fail
 * for a missing frontend feature rather than a startup defect. Port it together
 * with the frontend readiness contract in a later batch.
 */

const supportDir = path.dirname(fileURLToPath(import.meta.url));
export const desktopDir = path.resolve(supportDir, "..", "..");

/**
 * Resolve the startup artifact directory.
 *
 * `WDIO_LOG_DIR` is the runner's own log directory (`wdio.conf.mjs` logDir), so
 * evidence and service logs stay together; `READINESS_DIAGNOSTICS` is accepted
 * for compatibility with the recorded Windows recipes.
 */
export function resolveDiagnosticsDir(env = process.env, root = desktopDir) {
  const logDir = env.WDIO_LOG_DIR ?? env.READINESS_DIAGNOSTICS ?? "test-artifacts/wdio";
  return path.join(path.resolve(root, logDir), "startup");
}

export function ensureArtifactDir(env = process.env, root = desktopDir) {
  const dir = resolveDiagnosticsDir(env, root);
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}

export function writeArtifact(name, content, env = process.env, root = desktopDir) {
  const dir = ensureArtifactDir(env, root);
  const filePath = path.join(dir, name);
  if (typeof content === "object" && content !== null) {
    fs.writeFileSync(filePath, `${JSON.stringify(content, null, 2)}\n`, "utf8");
  } else {
    fs.writeFileSync(filePath, String(content), "utf8");
  }
  return filePath;
}

function describeError(error) {
  return error?.message ?? String(error);
}

function session(browser) {
  return browser ?? globalThis.browser;
}

const BLANK_DOC_URLS = new Set(["data:,", "about:blank", ""]);

/** True for the initial blank document a fresh WebView2 session reports. */
export function isBlankDocument(url) {
  if (url == null) {
    return true;
  }
  return BLANK_DOC_URLS.has(String(url).trim().toLowerCase());
}

/** A collected field that exists and did not fail to read. */
function isUsable(value) {
  return (
    typeof value === "string" && value.length > 0 && !value.startsWith("<unavailable:")
  );
}

/** True when collected evidence carries an XArchive-specific marker. */
export function isXArchiveDocument(evidence) {
  if (!evidence || evidence.error) {
    return false;
  }
  // A failed read is recorded as an `<unavailable: …>` string; requiring the
  // boolean keeps an unreadable DOM from being mistaken for a loaded document.
  if (evidence.rootExists === true) {
    return true;
  }
  if (isUsable(evidence.startupState) || isUsable(evidence.startupFallback)) {
    return true;
  }
  return (evidence.title ?? "").trim().toLowerCase() === "xarchive";
}

/** True when the evidence cannot be an XArchive document (blank target only). */
export function looksLikeApplicationDocument(evidence) {
  if (!evidence || evidence.error) {
    return false;
  }
  if (isXArchiveDocument(evidence)) {
    return true;
  }
  return !isBlankDocument(evidence.url);
}

export const WINDOW_TARGET_STATES = {
  NO_WINDOW_HANDLES: "NO_WINDOW_HANDLES",
  ONLY_BLANK_DOCUMENTS: "ONLY_BLANK_DOCUMENTS",
  APPLICATION_DOCUMENT_NOT_FOUND: "APPLICATION_DOCUMENT_NOT_FOUND",
  APPLICATION_DOCUMENT_FOUND: "APPLICATION_DOCUMENT_FOUND",
  WINDOW_TARGET_SWITCH_FAILED: "WINDOW_TARGET_SWITCH_FAILED",
};

/**
 * Collect URL, title, DOM readiness and startup markers for one window handle.
 * Restores the previously selected handle and never throws.
 */
export async function collectWindowEvidence(handleId, asCurrent = false, browser = session()) {
  const entry = { handle: handleId, current: asCurrent, switched: false, error: null };
  let previousHandle = null;
  try {
    previousHandle = await browser.getWindowHandle();
  } catch (error) {
    entry.error = `getWindowHandle: ${describeError(error)}`;
  }
  if (previousHandle !== handleId) {
    try {
      await browser.switchToWindow(handleId);
      entry.switched = true;
    } catch (error) {
      entry.error = `switchToWindow: ${describeError(error)}`;
      return entry;
    }
  }
  const collect = async (key, read) => {
    try {
      entry[key] = await read();
    } catch (error) {
      entry[key] = `<unavailable: ${describeError(error)}>`;
    }
  };
  await collect("url", () => browser.getUrl());
  await collect("title", () => browser.getTitle());
  await collect("readyState", () => browser.execute(() => document.readyState));
  await collect("rootExists", () => browser.execute(() => Boolean(document.getElementById("root"))));
  await collect("startupState", () =>
    browser.execute(() => document.documentElement?.dataset?.xarchiveStartup ?? ""),
  );
  await collect("startupFallback", () =>
    browser.execute(() => document.getElementById("startup-fallback")?.innerText ?? ""),
  );
  await collect("bodyTextPrefix", () =>
    browser.execute(() => document.body?.innerText?.slice(0, 2048) ?? ""),
  );
  await collect("pageSourceLength", async () => (await browser.getPageSource()).length);

  if (previousHandle && previousHandle !== handleId) {
    try {
      await browser.switchToWindow(previousHandle);
    } catch (error) {
      entry.restoreError = `restore window: ${describeError(error)}`;
    }
  }
  return entry;
}

/** Evidence for the currently selected window handle; never throws. */
export async function collectCurrentDocumentEvidence(browser = session()) {
  const evidence = {};
  const collect = async (key, read) => {
    try {
      evidence[key] = await read();
    } catch (error) {
      evidence[key] = `<unavailable: ${describeError(error)}>`;
    }
  };
  await collect("url", () => browser.getUrl());
  await collect("title", () => browser.getTitle());
  await collect("readyState", () => browser.execute(() => document.readyState));
  await collect("rootExists", () => browser.execute(() => Boolean(document.getElementById("root"))));
  await collect("rootText", () =>
    browser.execute(() => document.getElementById("root")?.innerText?.slice(0, 512) ?? ""),
  );
  await collect("startupState", () =>
    browser.execute(() => document.documentElement?.dataset?.xarchiveStartup ?? ""),
  );
  await collect("startupFallback", () =>
    browser.execute(() => document.getElementById("startup-fallback")?.innerText ?? ""),
  );
  return evidence;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Poll every window handle until one holds an XArchive document, recording the
 * full timeline so a failure shows what the session looked like over time.
 */
export async function discoverApplicationWindow({
  browser = session(),
  maxPollMs = Number(process.env.WDIO_STARTUP_DISCOVERY_TIMEOUT ?? 30000),
  pollMs = 400,
  sampleMs = 250,
  writeSnapshot = true,
  env = process.env,
  root = desktopDir,
} = {}) {
  const snapshot = {
    kind: "application-window-discovery",
    startedAt: Date.now(),
    discoveryTimeoutMs: maxPollMs,
    sampleIntervalMs: sampleMs,
    attempts: [],
    finalResult: null,
  };
  const deadline = Date.now() + maxPollMs;
  let sawHandles = false;

  while (Date.now() < deadline) {
    const attempt = {
      timestamp: Date.now(),
      elapsedMs: Date.now() - snapshot.startedAt,
      handleCount: 0,
      currentHandle: null,
      applicationCandidateCount: 0,
      onlyBlankDocuments: false,
    };
    let handles;
    try {
      handles = await browser.getWindowHandles();
    } catch (error) {
      attempt.error = `getWindowHandles: ${describeError(error)}`;
      snapshot.attempts.push(attempt);
      if (writeSnapshot) {
        writeArtifact("discovery.json", snapshot, env, root);
      }
      await sleep(pollMs);
      continue;
    }

    attempt.handleCount = handles.length;
    if (handles.length > 0) {
      sawHandles = true;
    }
    try {
      attempt.currentHandle = await browser.getWindowHandle();
    } catch (error) {
      attempt.currentHandle = `<unavailable: ${describeError(error)}>`;
    }

    const candidates = [];
    for (const handle of handles) {
      const evidence = await collectWindowEvidence(
        handle,
        handle === attempt.currentHandle,
        browser,
      );
      if (evidence.error && evidence.switched === false) {
        snapshot.attempts.push(attempt);
        snapshot.finalResult = {
          state: WINDOW_TARGET_STATES.WINDOW_TARGET_SWITCH_FAILED,
          failedHandle: handle,
          error: evidence.error,
        };
        if (writeSnapshot) {
          writeArtifact("discovery.json", snapshot, env, root);
        }
        return snapshot;
      }
      if (looksLikeApplicationDocument(evidence)) {
        candidates.push(evidence);
      }
    }
    attempt.applicationCandidateCount = candidates.length;
    attempt.onlyBlankDocuments = candidates.length === 0;
    snapshot.attempts.push(attempt);
    if (candidates.length > 0) {
      snapshot.finalResult = {
        state: WINDOW_TARGET_STATES.APPLICATION_DOCUMENT_FOUND,
        selectedHandle: candidates[0].handle,
        evidence: candidates[0],
      };
      if (writeSnapshot) {
        writeArtifact("discovery.json", snapshot, env, root);
      }
      return snapshot;
    }
    if (writeSnapshot) {
      writeArtifact("discovery.json", snapshot, env, root);
    }
    await sleep(sampleMs);
  }

  snapshot.finalResult = {
    state: sawHandles
      ? WINDOW_TARGET_STATES.ONLY_BLANK_DOCUMENTS
      : WINDOW_TARGET_STATES.NO_WINDOW_HANDLES,
    selectedHandle: null,
    evidence: null,
    lastAttempt: snapshot.attempts.at(-1) ?? null,
  };
  writeArtifact("discovery.json", snapshot, env, root);
  return snapshot;
}

/** Readiness gate: fail with the discovery timeline instead of a bare timeout. */
export async function waitForApplicationDocument(options = {}) {
  const env = options.env ?? process.env;
  const snapshot = await discoverApplicationWindow(options);
  const timeline = `${resolveDiagnosticsDir(env, options.root ?? desktopDir)}/discovery.json`;
  const state = snapshot.finalResult?.state;
  if (state === WINDOW_TARGET_STATES.APPLICATION_DOCUMENT_FOUND) {
    return snapshot.finalResult;
  }
  const reason =
    state === WINDOW_TARGET_STATES.ONLY_BLANK_DOCUMENTS
      ? `No XArchive application document found within ${snapshot.discoveryTimeoutMs}ms; ` +
        "every sampled window handle stayed on a blank document."
      : `Application window target could not be selected (${state}).`;
  const error = new Error(`${reason} Timeline: ${timeline}`);
  error.name = "WaitForApplicationDocumentError";
  error.state = state;
  error.timeline = snapshot;
  throw error;
}

/**
 * Record everything observable about a readiness failure. Never throws, so the
 * original error stays the failure the run reports.
 */
export async function captureReadinessFailure(
  label,
  error,
  {
    extraEvidence = null,
    screenshotName = "dashboard-startup-failure.png",
    browser = session(),
    env = process.env,
    root = desktopDir,
  } = {},
) {
  const failure = {
    label,
    timestamp: Date.now(),
    name: error?.name ?? "Error",
    message: error?.message ?? String(error),
    stack: error?.stack ?? null,
    url: null,
    readyState: null,
    startupState: null,
    rootExists: null,
    startupFallback: null,
    screenshotPath: null,
    extraEvidence,
  };
  try {
    Object.assign(failure, await collectCurrentDocumentEvidence(browser));
  } catch (collectError) {
    failure.collectError = describeError(collectError);
  }
  writeArtifact("failure.json", failure, env, root);
  try {
    const screenshotPath = path.join(ensureArtifactDir(env, root), screenshotName);
    await browser.saveScreenshot(screenshotPath);
    failure.screenshotPath = screenshotPath;
  } catch (screenshotError) {
    failure.screenshotError = describeError(screenshotError);
  }
  writeArtifact("failure.json", failure, env, root);
  try {
    writeArtifact("current-page.html", await browser.getPageSource(), env, root);
  } catch (sourceError) {
    writeArtifact(
      "current-page-error.json",
      { label, timestamp: Date.now(), error: describeError(sourceError) },
      env,
      root,
    );
  }
  console.error(
    "[xarchive-startup-failure]",
    label,
    failure.message,
    JSON.stringify({
      url: failure.url,
      readyState: failure.readyState,
      startupState: failure.startupState,
      rootExists: failure.rootExists,
    }),
  );
  return failure;
}

/**
 * Snapshot the session the moment it is created. The document is normally still
 * `data:,` at this point, which is exactly what makes the snapshot useful: a
 * later failure can be compared against it to tell whether the handle or URL
 * was empty from the start or drifted while waiting.
 */
export async function snapshotSessionStart(
  label = "session-start",
  browser = session(),
  { env = process.env, root = desktopDir } = {},
) {
  const snapshot = {
    kind: "session-start",
    label,
    timestamp: Date.now(),
    windowHandles: [],
    currentHandle: null,
    currentUrl: null,
    currentTitle: null,
    capabilities: null,
    error: null,
  };
  try {
    snapshot.windowHandles = await browser.getWindowHandles();
    snapshot.currentHandle = await browser.getWindowHandle();
    snapshot.currentUrl = await browser.getUrl().catch(() => "<unavailable>");
    snapshot.currentTitle = await browser.getTitle().catch(() => "<unavailable>");
    snapshot.capabilities = browser.capabilities ?? null;
  } catch (error) {
    snapshot.error = describeError(error);
  }
  writeArtifact("session-start.json", snapshot, env, root);
  console.log(
    "[xarchive-session-start]",
    label,
    JSON.stringify({
      windowHandles: snapshot.windowHandles.length,
      currentUrl: snapshot.currentUrl,
    }),
  );
  return snapshot;
}
