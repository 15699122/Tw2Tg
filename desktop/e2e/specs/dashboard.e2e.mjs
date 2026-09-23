import assert from "node:assert/strict";

import {
  waitForApplicationDocument,
  waitForStartupContract,
  captureReadinessFailure,
  collectCurrentDocumentEvidence,
  snapshotSessionStart,
} from "../support/native-startup.mjs";

async function waitForDashboard() {
  try {
    await waitForApplicationDocument();
    await waitForStartupContract();
  } catch (error) {
    await captureReadinessFailure("dashboard.startup.pre", error, {
      extraEvidence: { phase: "application-document-or-startup-contract", error: error?.message },
    });
    throw error;
  }
}

describe("XArchive Tauri desktop smoke", () => {
  before(async () => {
    // WQ-P0-WHITE-04C: 在等待应用文档之前记录 session 建立瞬间的初始
    // target 状态（此时文档通常仍是 data:,）。这份快照是 target-attachment
    // 诊断的第一份证据：如果后续发现失败，可以对照 session-start.json
    // 判断 handle/URL 是从会话一开始就为空，还是在等待期间偏离。
    await snapshotSessionStart("dashboard-before-hook");
    await waitForDashboard();
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
