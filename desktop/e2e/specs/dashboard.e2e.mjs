import assert from "node:assert/strict";

import {
  captureReadinessFailure,
  collectCurrentDocumentEvidence,
  snapshotSessionStart,
  waitForApplicationDocument,
  waitForStartupContract,
} from "../support/native-startup.mjs";

async function waitForDashboard() {
  await browser.waitUntil(
    async () => (await browser.$("h1").isExisting()) && (await browser.$("h1").isDisplayed()),
    {
      timeout: 20000,
      timeoutMsg: "XArchive dashboard heading did not become visible",
    },
  );
}

async function waitForDashboardWithEvidence() {
  try {
    // The readiness gate fails on a blank target instead of asserting on it, so
    // a session stuck on `data:,` reports the window-handle timeline rather than
    // a missing <h1> (WQ-ENG-09b). The frontend startup contract separates
    // "React never mounted" from "WebView2 served a blank document".
    await waitForApplicationDocument();
    await waitForStartupContract();
    await waitForDashboard();
  } catch (error) {
    await captureReadinessFailure("dashboard.readiness", error, {
      extraEvidence: { phase: "application-document-startup-contract-or-dashboard-heading" },
    });
    throw error;
  }
}

describe("XArchive Tauri desktop smoke", () => {
  before(async () => {
    // Record the target state at session start (usually still `data:,`) so a
    // later failure can be compared against it.
    await snapshotSessionStart("dashboard-before-hook");
    await waitForDashboardWithEvidence();
  });

  it("completes the frontend startup contract before rendering the dashboard", async () => {
    const evidence = await collectCurrentDocumentEvidence();
    assert.equal(evidence.readyState, "complete");
    assert.equal(evidence.rootExists, true);
    assert.equal(evidence.startupState, "react_mount_completed");
    assert.equal(evidence.startupFallback, "");
  });

  it("renders the dashboard shell in the native window", async () => {
    assert.equal(await browser.$("h1").getText(), "工作台");
    assert.equal(await browser.$("main").isDisplayed(), true);
    assert.equal(await browser.$('[aria-label="主导航"]').isExisting(), true);
  });

  it("exposes the stable dashboard regions used by Windows checks", async () => {
    assert.equal(await browser.$('[aria-label="归档概览"]').isExisting(), true);
    assert.equal(await browser.$("button").isExisting(), true);
    assert.equal(await browser.$("body").isDisplayed(), true);
  });
});