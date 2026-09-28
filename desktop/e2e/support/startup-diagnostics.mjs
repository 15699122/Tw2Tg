import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Startup failure evidence for the Tauri WebDriver E2E suite.
 *
 * A session that never leaves `data:,` is ambiguous on its own: it can mean the
 * app never navigated, the session bound to a blank WebView, or the
 * msedgedriver / WebView2 runtime pairing was wrong. Recording window handles,
 * URLs, titles and screenshots turns the next failure into evidence instead of
 * a hand-written temporary spec (2026-09-28 WQ-ENG-09b).
 *
 * The module is best effort by contract: diagnostics must never replace or
 * mask the failure that triggered them, and they must not change the outcome
 * of the run.
 */

const supportDir = path.dirname(fileURLToPath(import.meta.url));
export const desktopDir = path.resolve(supportDir, "..", "..");

/** Resolve the startup artifact directory, mirroring `wdio.conf.mjs` logDir. */
export function resolveDiagnosticsDir(env = process.env, root = desktopDir) {
  const logDir = env.WDIO_LOG_DIR
    ? path.resolve(root, env.WDIO_LOG_DIR)
    : path.resolve(root, "test-artifacts", "wdio");
  return path.join(logDir, "startup");
}

function describeError(error) {
  return error?.message ?? String(error);
}

function timestampSlug(date) {
  return date.toISOString().replace(/[:.]/g, "-");
}

/**
 * Enumerate every window handle with its URL and title. Every step is guarded
 * so a broken session produces an error entry instead of an exception, and the
 * originally selected window is restored at the end.
 */
export async function collectWindowEvidence(browser) {
  const evidence = { currentHandle: null, windows: [], errors: [] };
  if (!browser) {
    evidence.errors.push("no browser session object");
    return evidence;
  }

  try {
    evidence.currentHandle = await browser.getWindowHandle();
  } catch (error) {
    evidence.errors.push(`getWindowHandle: ${describeError(error)}`);
  }

  let handles = [];
  try {
    handles = await browser.getWindowHandles();
  } catch (error) {
    evidence.errors.push(`getWindowHandles: ${describeError(error)}`);
    return evidence;
  }

  for (const handle of handles) {
    const entry = {
      handle,
      current: handle === evidence.currentHandle,
      url: null,
      title: null,
    };
    try {
      await browser.switchToWindow(handle);
    } catch (error) {
      entry.error = `switchToWindow: ${describeError(error)}`;
      evidence.errors.push(entry.error);
      evidence.windows.push(entry);
      continue;
    }
    try {
      entry.url = await browser.getUrl();
    } catch (error) {
      entry.error = `getUrl: ${describeError(error)}`;
    }
    try {
      entry.title = await browser.getTitle();
    } catch (error) {
      entry.error = [entry.error, `getTitle: ${describeError(error)}`]
        .filter(Boolean)
        .join("; ");
    }
    evidence.windows.push(entry);
  }

  if (evidence.currentHandle) {
    try {
      await browser.switchToWindow(evidence.currentHandle);
    } catch (error) {
      evidence.errors.push(`restore window: ${describeError(error)}`);
    }
  }
  return evidence;
}

/** Collect window evidence and persist it as JSON plus per-window screenshots. */
export async function writeStartupDiagnostics({
  browser,
  reason,
  env = process.env,
  root = desktopDir,
  date = new Date(),
} = {}) {
  const dir = resolveDiagnosticsDir(env, root);
  fs.mkdirSync(dir, { recursive: true });
  const evidence = await collectWindowEvidence(browser);

  for (const entry of evidence.windows) {
    try {
      if (!entry.current) {
        await browser.switchToWindow(entry.handle);
      }
      const file = path.join(dir, `window-${entry.handle}.png`);
      await browser.saveScreenshot(file);
      entry.screenshot = path.basename(file);
    } catch (error) {
      evidence.errors.push(`saveScreenshot(${entry.handle}): ${describeError(error)}`);
    }
  }

  const evidencePath = path.join(dir, `startup-diagnostics-${timestampSlug(date)}.json`);
  fs.writeFileSync(
    evidencePath,
    `${JSON.stringify({ reason: reason ?? null, capturedAt: date.toISOString(), ...evidence }, null, 2)}\n`,
    "utf8",
  );
  return { evidencePath, dir, windows: evidence.windows, errors: evidence.errors };
}

/** Never-throwing wrapper used by the E2E readiness hooks. */
export async function captureStartupDiagnostics(options) {
  try {
    return await writeStartupDiagnostics(options);
  } catch (error) {
    console.warn(`[wdio] startup diagnostics unavailable: ${describeError(error)}`);
    return null;
  }
}
