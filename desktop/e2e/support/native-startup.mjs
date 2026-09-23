import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const supportDir = path.dirname(fileURLToPath(import.meta.url));
const desktopRoot = path.resolve(supportDir, "..");

function artifactDir() {
  return path.resolve(desktopRoot, process.env.READINESS_DIAGNOSTICS ?? "test-artifacts/wdio");
}
function startupDir() {
  return path.join(artifactDir(), "startup");
}

export const diagnosticsRoot = artifactDir;
export const startupArtifactDir = startupDir;

export function ensureArtifactDir() {
  const seen = new Set();
  for (const d of [artifactDir(), startupDir()]) {
    if (seen.has(d)) continue;
    seen.add(d);
    fs.mkdirSync(d, { recursive: true });
  }
  return startupDir();
}

export function writeArtifact(name, content) {
  ensureArtifactDir();
  const filePath = path.join(startupDir(), name);
  if (typeof content === "object" && content !== null) fs.writeFileSync(filePath, JSON.stringify(content, null, 2), "utf8");
  else fs.writeFileSync(filePath, String(content), "utf8");
  return filePath;
}

const BLANK_DOC_URLS = new Set(["data:,", "about:blank", ""]);

export function isBlankDocument(url) {
  if (url == null) return true;
  return BLANK_DOC_URLS.has(String(url).trim().toLowerCase());
}

export function isXArchiveDocument(evidence) {
  if (!evidence || evidence.error) return false;
  if (evidence.rootExists) return true;
  const s = evidence.startupState ?? "";
  if (typeof s === "string" && s.length > 0) return true;
  if (typeof evidence.startupFallback === "string" && evidence.startupFallback.length > 0) return true;
  if ((evidence.title ?? "").trim().toLowerCase() === "xarchive") return true;
  return false;
}

export function looksLikeApplicationDocument(evidence) {
  if (!evidence || evidence.error) return false;
  if (isXArchiveDocument(evidence)) return true;
  if (!isBlankDocument(evidence.url)) return true;
  return false;
}

const WINDOW_STATE = {
  NO_WINDOW_HANDLES: "NO_WINDOW_HANDLES",
  ONLY_BLANK_DOCUMENTS: "ONLY_BLANK_DOCUMENTS",
  APPLICATION_DOCUMENT_NOT_FOUND: "APPLICATION_DOCUMENT_NOT_FOUND",
  APPLICATION_DOCUMENT_FOUND: "APPLICATION_DOCUMENT_FOUND",
  WINDOW_TARGET_SWITCH_FAILED: "WINDOW_TARGET_SWITCH_FAILED",
};

export const WINDOW_TARGET_STATES = WINDOW_STATE;

export async function collectWindowEvidence(handleId, asCurrent = false) {
  const previousHandle = await browser.getWindowHandle();
  let switched = false;
  if (previousHandle !== handleId) {
    try {
      await browser.switchToWindow(handleId);
      switched = true;
    } catch (error) {
      return { handle: handleId, current: asCurrent, switched, error: `switchToWindow failed: ${error?.message ?? String(error)}` };
    }
  }
  try {
    return {
      handle: handleId,
      current: asCurrent,
      switched,
      url: await browser.getUrl(),
      title: await browser.getTitle(),
      readyState: await browser.execute(() => document.readyState),
      rootExists: await browser.execute(() => Boolean(document.getElementById("root"))),
      startupState: await browser.execute(() => document.documentElement?.dataset?.xarchiveStartup ?? ""),
      startupFallback: await browser.execute(() => document.getElementById("startup-fallback")?.innerText ?? ""),
      bodyTextPrefix: await browser.execute(() => document.body?.innerText?.slice(0, 2048) ?? ""),
      pageSourceLength: await browser.getPageSource().then((s) => s.length, () => -1),
      error: null,
    };
  } catch (error) {
    return { handle: handleId, current: asCurrent, switched, error: `evidence collection failed: ${error?.message ?? String(error)}` };
  } finally {
    if (switched) {
      try { await browser.switchToWindow(previousHandle); } catch {}
    }
  }
}

export async function collectCurrentDocumentEvidence() {
  let url, readyState, startupState, rootExists, rootText, startupFallback;
  try { url = await browser.getUrl(); } catch { url = "<unavailable>"; }
  try { readyState = await browser.execute(() => document.readyState); } catch { readyState = "<unavailable>"; }
  try { startupState = await browser.execute(() => document.documentElement?.dataset?.xarchiveStartup ?? ""); } catch { startupState = "<unavailable>"; }
  try { rootExists = await browser.execute(() => Boolean(document.getElementById("root"))); } catch { rootExists = false; }
  try { rootText = await browser.execute(() => document.getElementById("root")?.innerText?.slice(0, 512) ?? ""); } catch { rootText = "<unavailable>"; }
  try { startupFallback = await browser.execute(() => document.getElementById("startup-fallback")?.innerText ?? ""); } catch { startupFallback = "<unavailable>"; }
  return { url, readyState, startupState, rootExists, rootText, startupFallback };
}

export async function currentDocumentTitle() {
  try { return await browser.getTitle(); } catch { return "<unavailable>"; }
}

export async function discoverApplicationWindow({ maxPollMs = Number(process.env.WDIO_STARTUP_DISCOVERY_TIMEOUT ?? 30000), pollMs = 400, sampleMs = 250, writeSnapshot = true } = {}) {
  const snapshot = { kind: "application-window-discovery", startedAt: Date.now(), discoveryTimeoutMs: maxPollMs, pollIntervalMs: pollMs, sampleIntervalMs: sampleMs, attempts: [], finalResult: null };
  const deadline = Date.now() + maxPollMs;
  while (Date.now() < deadline) {
    const attempt = { timestamp: Date.now(), elapsedMs: Date.now() - snapshot.startedAt, handleCount: 0, currentHandle: null, handles: [], urlAtStart: await browser.getUrl().catch(() => "<unavailable>") };
    let handles;
    try { handles = await browser.getWindowHandles(); } catch (error) { attempt.error = `getWindowHandles failed: ${error?.message ?? String(error)}`; attempt.handlesError = true; snapshot.attempts.push(attempt); if (writeSnapshot) writeArtifact("discovery.json", snapshot); await sleep(pollMs); continue; }
    attempt.handleCount = handles.length;
    attempt.handles = handles.map((h) => h ?? "<unavailable>");
    let currentHandle;
    try { currentHandle = await browser.getWindowHandle(); } catch { currentHandle = "<unavailable>"; }
    attempt.currentHandle = currentHandle;
    const candidates = [];
    for (const handle of handles) {
      const evidence = await collectWindowEvidence(handle, handle === currentHandle);
      if (evidence.error && evidence.switched === false) {
        snapshot.attempts.push(attempt);
        snapshot.finalResult = { state: WINDOW_STATE.WINDOW_TARGET_SWITCH_FAILED, failedHandle: handle, error: evidence.error };
        if (writeSnapshot) writeArtifact("discovery.json", snapshot);
        return snapshot;
      }
      if (looksLikeApplicationDocument(evidence)) candidates.push({ handle, evidence, rootExists: evidence.rootExists, startupState: evidence.startupState });
    }
    attempt.applicationCandidate = candidates[0] ?? null;
    attempt.applicationCandidateCount = candidates.length;
    attempt.onlyBlankDocuments = candidates.length === 0;
    snapshot.attempts.push(attempt);
    if (candidates.length > 0) {
      snapshot.finalResult = { state: WINDOW_STATE.APPLICATION_DOCUMENT_FOUND, selectedHandle: candidates[0].handle, evidence: candidates[0].evidence };
      if (writeSnapshot) writeArtifact("discovery.json", snapshot);
      return snapshot;
    }
    if (writeSnapshot) writeArtifact("discovery.json", snapshot);
    await sleep(sampleMs);
  }
  snapshot.finalResult = { state: WINDOW_STATE.ONLY_BLANK_DOCUMENTS, selectedHandle: null, evidence: null, lastAttempt: snapshot.attempts.at(-1) ?? null };
  writeArtifact("discovery.json", snapshot);
  return snapshot;
}

export async function WaitForApplicationDocumentError(message, timeline) {
  const error = new Error(message);
  error.name = "WaitForApplicationDocumentError";
  error.timeline = timeline;
  return error;
}

export async function waitForApplicationDocument({ maxPollMs = Number(process.env.WDIO_STARTUP_DISCOVERY_TIMEOUT ?? 30000), ...discoveryOptions } = {}) {
  const snapshot = await discoverApplicationWindow({ maxPollMs, ...discoveryOptions });
  switch (snapshot.finalResult?.state) {
    case WINDOW_STATE.APPLICATION_DOCUMENT_FOUND: return snapshot.finalResult;
    case WINDOW_STATE.ONLY_BLANK_DOCUMENTS: throw await WaitForApplicationDocumentError(`No XArchive application document found within ${maxPollMs}ms. Session remained on a blank document (current URL still blank) across all handle samples. Timeline: ${startupArtifactDir()}/discovery.json`, snapshot);
    case WINDOW_STATE.NO_WINDOW_HANDLES:
    case WINDOW_STATE.APPLICATION_DOCUMENT_NOT_FOUND:
    case WINDOW_STATE.WINDOW_TARGET_SWITCH_FAILED: throw await WaitForApplicationDocumentError(`Application window target could not be selected: ${snapshot.finalResult?.state}. Timeline: ${startupArtifactDir()}/discovery.json`, snapshot);
    default: throw await WaitForApplicationDocumentError(`Unexpected application-window discovery result: ${snapshot.finalResult ?? null}. Timeline: ${startupArtifactDir()}/discovery.json`, snapshot);
  }
}

export async function waitForStartupContract({ maxPollMs = Number(process.env.WDIO_STARTUP_CONTRACT_TIMEOUT ?? 25000), pollMs = 200 } = {}) {
  const snapshot = { kind: "startup-contract", startedAt: Date.now(), contractTimeoutMs: maxPollMs, pollIntervalMs: pollMs, attempts: [], finalResult: null };
  const deadline = Date.now() + maxPollMs;
  while (Date.now() < deadline) {
    const evidence = await collectCurrentDocumentEvidence();
    const attempt = { timestamp: Date.now(), elapsedMs: Date.now() - snapshot.startedAt, evidence, rootExists: evidence.rootExists, startupState: evidence.startupState, fallbackEmpty: evidence.startupFallback === "", contractOk: evidence.rootExists === true && evidence.startupState === "react_mount_completed" && evidence.startupFallback === "" };
    snapshot.attempts.push(attempt);
    if (attempt.contractOk) { snapshot.finalResult = { satisfied: true, evidence }; writeArtifact("contract.json", snapshot); return snapshot; }
    writeArtifact("contract.json", snapshot);
    const remaining = Math.max(0, deadline - Date.now());
    const sleepMs = Math.min(pollMs, remaining);
    if (sleepMs <= 0) break;
    await sleep(sleepMs);
  }
  snapshot.finalResult = { satisfied: false, evidence: snapshot.attempts.at(-1)?.evidence ?? null, lastAttempt: snapshot.attempts.at(-1) ?? null };
  writeArtifact("contract.json", snapshot);
  return snapshot;
}

export async function captureReadinessFailure(label, error, { extraEvidence = null, screenshotName = "dashboard-startup-failure.png" } = {}) {
  const failure = { label, timestamp: Date.now(), name: error?.name ?? "Error", message: error?.message ?? String(error), stack: error?.stack ?? null, url: null, readyState: null, startupState: null, rootExists: null, startupFallback: null, screenshotPath: null, screenshotError: null, extraEvidence: extraEvidence ?? null };
  try {
    const evidence = await collectCurrentDocumentEvidence();
    failure.url = evidence.url; failure.readyState = evidence.readyState; failure.startupState = evidence.startupState; failure.rootExists = evidence.rootExists; failure.startupFallback = evidence.startupFallback;
  } catch (collectError) { failure.collectError = collectError?.message ?? String(collectError); }
  ensureArtifactDir();
  writeArtifact("failure.json", failure);
  const screenshotTarget = path.join(startupDir(), screenshotName);
  try { await browser.saveScreenshot(screenshotTarget); failure.screenshotPath = screenshotTarget; } catch (screenshotError) { failure.screenshotError = screenshotError?.message ?? String(screenshotError); }
  writeArtifact("failure.json", failure);
  try { writeArtifact("current-page.html", await browser.getPageSource()); } catch (sourceError) { writeArtifact("current-page-error.json", { label, timestamp: Date.now(), error: sourceError?.message ?? String(sourceError) }); }
  console.error("[xarchive-startup-failure]", label, failure.message, JSON.stringify({ url: failure.url, readyState: failure.readyState, startupState: failure.startupState, rootExists: failure.rootExists }));
  return failure;
}

function sleep(ms) { return new Promise((resolve) => setTimeout(resolve, ms)); }

export async function snapshotSessionStart(label = "session-start") {
  const snapshot = {
    kind: "session-start",
    label,
    timestamp: Date.now(),
    windowHandles: [],
    currentHandle: null,
    currentUrl: null,
    currentTitle: null,
    capabilities: null,
    error: null
  };
  try {
    snapshot.windowHandles = await browser.getWindowHandles();
    snapshot.currentHandle = await browser.getWindowHandle();
    snapshot.currentUrl = await browser.getUrl().catch(() => "<unavailable>");
    snapshot.currentTitle = await browser.getTitle().catch(() => "<unavailable>");
    snapshot.capabilities = browser.capabilities ?? null;
  } catch (error) {
    snapshot.error = error?.message ?? String(error);
  }
  ensureArtifactDir();
  writeArtifact("session-start.json", snapshot);
  console.log("[xarchive-session-start]", label, JSON.stringify({ windowHandles: snapshot.windowHandles.length, currentUrl: snapshot.currentUrl }));
  return snapshot;
}

