import { describe, it } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import {
  captureStartupDiagnostics,
  collectWindowEvidence,
  resolveDiagnosticsDir,
  writeStartupDiagnostics,
} from "../e2e/support/startup-diagnostics.mjs";

function tempRoot() {
  return fs.mkdtempSync(path.join(os.tmpdir(), "xarchive-startup-diagnostics-"));
}

function fakeBrowser(windows, { failScreenshots = false } = {}) {
  const calls = [];
  let current = windows[0]?.handle ?? null;
  return {
    calls,
    async getWindowHandle() {
      return current;
    },
    async getWindowHandles() {
      return windows.map((window) => window.handle);
    },
    async switchToWindow(handle) {
      calls.push(`switch:${handle}`);
      const found = windows.find((window) => window.handle === handle);
      if (!found) {
        throw new Error(`no such window ${handle}`);
      }
      current = handle;
    },
    async getUrl() {
      return windows.find((window) => window.handle === current)?.url ?? null;
    },
    async getTitle() {
      return windows.find((window) => window.handle === current)?.title ?? null;
    },
    async saveScreenshot(file) {
      calls.push(`shot:${file}`);
      if (failScreenshots) {
        throw new Error("screenshot unsupported");
      }
      fs.writeFileSync(file, "png");
    },
  };
}

describe("startup diagnostics", () => {
  it("resolves the artifact directory from WDIO_LOG_DIR and the default", () => {
    const root = path.join(path.sep, "project", "desktop");
    assert.equal(
      resolveDiagnosticsDir({ WDIO_LOG_DIR: "custom-logs" }, root),
      path.join(root, "custom-logs", "startup"),
    );
    assert.equal(
      resolveDiagnosticsDir({}, root),
      path.join(root, "test-artifacts", "wdio", "startup"),
    );
  });

  it("records url and title per window handle and restores the current window", async () => {
    const browser = fakeBrowser([
      { handle: "W1", url: "data:,", title: "" },
      { handle: "W2", url: "http://tauri.localhost", title: "XArchive" },
    ]);
    const evidence = await collectWindowEvidence(browser);
    assert.equal(evidence.currentHandle, "W1");
    assert.deepEqual(
      evidence.windows.map((window) => window.url),
      ["data:,", "http://tauri.localhost"],
    );
    assert.deepEqual(
      evidence.windows.map((window) => window.title),
      ["", "XArchive"],
    );
    assert.equal(browser.calls.at(-1), "switch:W1");
  });

  it("keeps collecting when one window query fails", async () => {
    const browser = fakeBrowser([{ handle: "W1", url: "data:,", title: "" }]);
    browser.getTitle = async () => {
      throw new Error("title unavailable");
    };
    const evidence = await collectWindowEvidence(browser);
    assert.equal(evidence.windows.length, 1);
    assert.match(evidence.windows[0].error, /title unavailable/);
  });

  it("writes evidence and a screenshot for a blank session", async () => {
    const root = tempRoot();
    const browser = fakeBrowser([{ handle: "W1", url: "data:,", title: "" }]);
    const date = new Date("2026-09-28T02:00:00.000Z");
    const result = await writeStartupDiagnostics({
      browser,
      reason: "dashboard readiness timeout",
      env: { WDIO_LOG_DIR: "test-artifacts/wdio" },
      root,
      date,
    });
    const evidence = JSON.parse(fs.readFileSync(result.evidencePath, "utf8"));
    assert.equal(evidence.reason, "dashboard readiness timeout");
    assert.equal(evidence.capturedAt, date.toISOString());
    assert.equal(evidence.windows[0].url, "data:,");
    assert.equal(evidence.windows[0].screenshot, "window-W1.png");
    assert.equal(fs.existsSync(path.join(result.dir, "window-W1.png")), true);
    assert.equal(
      path.basename(result.evidencePath),
      "startup-diagnostics-2026-09-28T02-00-00-000Z.json",
    );
    fs.rmSync(root, { recursive: true, force: true });
  });

  it("records a missing session instead of throwing", async () => {
    const root = tempRoot();
    const result = await captureStartupDiagnostics({ browser: null, env: {}, root });
    assert.deepEqual(result.windows, []);
    assert.deepEqual(result.errors, ["no browser session object"]);
    fs.rmSync(root, { recursive: true, force: true });
  });

  it("records screenshot failures without failing the capture", async () => {
    const root = tempRoot();
    const result = await captureStartupDiagnostics({
      browser: fakeBrowser([{ handle: "W1", url: "data:,", title: "" }], { failScreenshots: true }),
      reason: "plugin spec dashboard readiness timeout",
      env: {},
      root,
    });
    assert.equal(result.windows[0].screenshot, undefined);
    assert.ok(result.errors.some((error) => /saveScreenshot/.test(error)));
    fs.rmSync(root, { recursive: true, force: true });
  });
});
